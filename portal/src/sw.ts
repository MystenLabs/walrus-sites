import {
    getFullnodeUrl,
    SuiClient,
    SuiObjectData,
} from "@mysten/sui.js/client";
import * as baseX from "base-x";
import {
    fromB64,
    fromHEX,
    isValidSuiObjectId,
    toHEX,
} from "@mysten/sui.js/utils";
import { SITE_NAMES, NETWORK, DOMAIN } from "./constants";
import { bcs } from "@mysten/sui.js/bcs";
import template_404 from "../static/404-page.template.html";
import error_page_style from "../static/error-page-style.html";

// This is to get TypeScript to recognize `clients` and `self`
// Default type of `self` is `WorkerGlobalScope & typeof globalThis`
// https://github.com/microsoft/TypeScript/issues/14877
declare var self: ServiceWorkerGlobalScope;
declare var clients: Clients;

var BASE36 = "0123456789abcdefghijklmnopqrstuvwxyz";
const b36 = baseX(BASE36);

self.addEventListener("install", (event) => {
    self.skipWaiting();
});

self.addEventListener("activate", (event) => {
    clients.claim();
});

self.addEventListener("fetch", (event) => {
    const url = event.request.url;
    const scope = self.registration.scope;

    // Check if the request is for a blocksite
    const parsedUrl = getSubdomainAndPath(url);
    console.log("Parsed URL: ", parsedUrl);
    if (parsedUrl && parsedUrl.subdomain) {
        event.respondWith(resolveAndFetchPage(parsedUrl));
        return;
    }

    // Handle the case in which we are at the root `BASE_URL`
    if (url === scope || url === scope + "index.html") {
        const newUrl = scope + "index-sw-enabled.html";
        event.respondWith(fetch(newUrl));
        return;
    }

    // Default case: Fetch all other sites from the web
    return fetch(event.request).then((response) => {
        return response;
    });
});

// Subdomain encoding & parsing //

// Use base36 instead of HEX to encode object ids in the subdomain, as the subdomain must be < 64 characters.
// The encoding must be case insensitive.

function subdomainToObjectId(subdomain: string): string | null {
    const objectId = "0x" + toHEX(b36.decode(subdomain.toLowerCase()));
    return isValidSuiObjectId(objectId) ? objectId : null;
}

type Path = {
    subdomain: string;
    path: string;
};

function getSubdomainAndPath(scope: string): Path | null {
    // At the moment we only support one subdomain level.
    const url = new URL(scope);
    const hostname = url.hostname.split(".");

    if (
        hostname.length === 3 ||
        (hostname.length === 2 && hostname[1] === "localhost")
    ) {
        // Accept only one level of subdomain
        // eg `subdomain.example.com` or `subdomain.localhost` in case of local development
        const path =
            url.pathname == "/" ? "/index.html" : removeLastSlash(url.pathname);
        return { subdomain: hostname[0], path } as Path;
    }
    return null;
}

/** Remove the last forward-slash if present
 * Resources on chain are only stored as `/path/to/resource.extension`.
 */
function removeLastSlash(path: string): string {
    return path.endsWith("/") ? path.slice(0, -1) : path;
}

// SuiNS-like functionality //
// Right now this just matches the subdomain to fixed strings. Should be replaced with a SuiNS lookup.

/**  Resolve the subdomain to an object ID using SuiNS
The subdomain `example` will look up `example.sui` and return the object ID if found. */
async function resolveSuiNsAddress(
    client: SuiClient,
    subdomain: string
): Promise<string | null> {
    const suiObjectId: string = await client.call(
        "suix_resolveNameServiceAddress",
        [subdomain + ".sui"]
    );
    return suiObjectId ? suiObjectId : null;
}

function hardcodedSubdmains(subdomain: string): string | null {
    if (subdomain in SITE_NAMES) {
        return SITE_NAMES[subdomain];
    }
    return null;
}

// Types & encoding/decoding for the on-chain objects //

// Fetching objects from the full node //
type BlockResource = {
    name: string;
    created: number;
    updated: number | null;
    version: number;
    content_type: string;
    content_encoding: string;
    parts: number;
    contents: Uint8Array;
};

// define UID as a 32-byte array, then add a transform to/from hex strings
const UID = bcs.fixedArray(32, bcs.u8()).transform({
    input: (id: string) => fromHEX(id),
    output: (id) => toHEX(Uint8Array.from(id)),
});

const NUMBER = bcs.u64().transform({
    input: (ts: number) => Number(ts),
    output: (ts) => Number(ts),
});

const VECTOR = bcs.vector(bcs.u8()).transform({
    input: (contents: Uint8Array) => Array.from(contents),
    output: (contents) => Uint8Array.from(contents),
});

const BlockPageStruct = bcs.struct("BlockPage", {
    name: bcs.string(),
    created: NUMBER,
    updated: bcs.option(NUMBER),
    version: NUMBER,
    content_type: bcs.string(),
    content_encoding: bcs.string(),
    parts: NUMBER,
    contents: VECTOR,
});

const FieldStruct = bcs.struct("Field", {
    id: UID,
    name: VECTOR,
    value: BlockPageStruct,
});

// Fectching & decompressing on-chain data //

async function resolveAndFetchPage(parsedUrl: Path): Promise<Response> {
    const rpcUrl = getFullnodeUrl(NETWORK);
    const client = new SuiClient({ url: rpcUrl });

    let objectId = hardcodedSubdmains(parsedUrl.subdomain);
    if (!objectId) {
        // Try to convert the subdomain to an object ID
        // NOTE: This effectively _disables_ any SuiNs name that is
        // the base36 encoding of an object ID (i.e., a 32-byte
        // string). This is desirable, prevents people from getting
        // suins names that are the base36 encoding the object ID of a
        // target blocksite (with the goal of hijacking non-suins
        // queries)
        objectId = subdomainToObjectId(parsedUrl.subdomain);
    }
    if (!objectId) {
        // Check if there is a SuiNs name
        try {
            objectId = await resolveSuiNsAddress(client, parsedUrl.subdomain);
        } catch {
            return fullNodeFail();
        }
    }
    if (objectId) {
        console.log("Object ID: ", objectId);
        console.log(
            "Base36 version of the object ID: ",
            b36.encode(fromHEX(objectId))
        );
        return fetchPage(client, objectId, parsedUrl.path);
    }
    return noObjectIdFound();
}

async function loadBlocklist(): Promise<Set<string>> {
    console.log("Fetching blocklist from network ...");

    const response = await fetch(`http://${DOMAIN}/blocklist.txt`);
    if (!response.ok) {
        console.log("Failed to fetch the blocklist: ", response);
    }
    const blocklist = await response.text().then((body) => {
        return new Set<string>(
            body.split(/\r?\n/)
                .filter((entry) => entry.trim().length !== 0)
        );
    });
    console.log("Loaded list of blocked objects. Count = ", blocklist.size);

    return blocklist;
}

/** Fetch the page */
async function fetchPage(
    client: SuiClient,
    objectId: string,
    path: string
): Promise<Response> {
    const blocklist = await loadBlocklist();
    if (blocklist.has(objectId)) {
        return objectBlocked();
    }

    const [blockResource, isBlocked] = await fetchBlockResource(client, objectId, path, blocklist);
    if (isBlocked) {
        return objectBlocked();
    }
    if (!blockResource || !blockResource.contents) {
        return siteNotFound();
    }
    let contents = blockResource.contents;

    //  Either its a one-part page, or we need to fetch the other parts
    if (blockResource.parts >= 1) {
        // Fetch the other parts in parallel
        const otherParts = await Promise.all([
            ...partNames(path, blockResource.parts).map((part_name) =>
                fetchBlockResource(client, objectId, part_name, blocklist)
            ),
        ]);


        let contentsArray = [blockResource.contents];
        for (const [resource, isBlocked] of otherParts) {
            if (isBlocked) {
                return objectBlocked();
            } else if (!blockResource || !blockResource.contents) {
                return siteNotFound();
            }
            contentsArray.push(blockResource.contents);
        }

        contents = Uint8Array.from(contentsArray.reduce((a, b) => [...a, ...b], []));
    }
    const decompressed = await decompressData(
        contents,
        blockResource.content_encoding
    );
    if (!decompressed) {
        return siteNotFound();
    }
    return new Response(decompressed, {
        headers: {
            "Content-Type": blockResource.content_type,
        },
    });
}

function partNames(name: string, parts: number): string[] {
    return Array.from({ length: parts - 1 }, (_, i) => `part-${i + 1}${name}`);
}

async function fetchBlockResource(
    client: SuiClient,
    objectId: string,
    path: string,
    blocklist: Set<string>
): Promise<[BlockResource | null, boolean]> {
    const dynamicFields = await client.getDynamicFieldObject({
        parentId: objectId,
        name: { type: "0x1::string::String", value: path },
    });
    console.log("Dynamic fields: ", dynamicFields);
    if (!dynamicFields.data) {
        console.log("No dynamic field found");
        console.log(dynamicFields);
        return [null, false];
    }
    if (blocklist.has(dynamicFields.data.objectId)) {
        console.log("object dynamic field is blocked:", path);
        return [null, true];
    }

    const pageData = await client.getObject({
        id: dynamicFields.data.objectId,
        options: { showBcs: true },
    });
    if (!pageData.data) {
        console.log("No page data found");
        return [null, false];
    }

    const blockPage = getPageFields(pageData.data);
    if (!blockPage || !blockPage.contents) {
        return [null, false];
    }
    return [blockPage, false];
}

// Type definitions for BCS decoding //
function getPageFields(data: SuiObjectData): BlockResource | null {
    // Deserialize the bcs encoded struct
    if (data.bcs && data.bcs.dataType === "moveObject") {
        const blockpage = FieldStruct.parse(fromB64(data.bcs.bcsBytes));
        return blockpage.value;
    }
    return null;
}

async function decompressData(
    data: ArrayBuffer,
    contentEncoding: string
): Promise<ArrayBuffer | null> {
    if (contentEncoding === "plaintext") {
        return data;
    }
    // check that contentencoding is a valid CompressionFormat
    if (["gzip", "deflate", "deflate-raw"].includes(contentEncoding)) {
        const enc = contentEncoding as CompressionFormat;
        const blob = new Blob([data], { type: "application/gzip" });
        const stream = blob.stream().pipeThrough(new DecompressionStream(enc));
        const response = await new Response(stream).arrayBuffer().catch((e) => {
            console.error("DecompressionStream error", e);
        });
        if (response) return response;
    }
    return null;
}

function objectBlocked(): Response {
    return ErrorResponse(
        418,
        "418 I'm a Teapot",
        "418",
        ("I'm a teapot, so I can't serve you that <em>particular</em>" +
         " Blocksite.<br/><br/>Some may say that a \"410 Gone\" or a \"403 Forbidden\" would be more" +
         " appropriate, but to them I say \"Shhhh! You're not a teapot!\"")
    );
}

function siteNotFound(): Response {
    return Response404(
        "The URL provided points to an object ID, but the object does not seem to be a blocksite."
    );
}

function noObjectIdFound(): Response {
    return Response404("The URL provided does not point to a valid object id.");
}

function fullNodeFail(): Response {
    return Response404("Failed to contact the full node.");
}

function ErrorResponse(status_code: number, title: string, headline: string, message: string): Response {
    const html = `
        <!DOCTYPE html>
        <html lang="en">
        <head>
            <meta charset="UTF-8" />
            <meta name="viewport" content="width=device-width, initial-scale=1.0" />
            <title>${ title }</title>
            ${ error_page_style }
        </head>
        <body>
            <div class="container">
                <p class="headline error">${ headline }</p>
                <p class="description">${ message }</p>
                <button onclick="history.back()" href="/" class="button">Go Back</button>
            </div>
        </body>
        </html>`;
        return new Response(
            html,
            {
                status: status_code,
                headers: { "Content-Type": "text/html" },
            }
        );
}

function Response404(message: string): Response {
    return ErrorResponse(404, "404 Blocksite Not Found", "404", message);
}
