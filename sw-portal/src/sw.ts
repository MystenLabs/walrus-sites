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
import { SITE_NAMES, NETWORK, BASE_URL } from "./constants";
import { bcs } from "@mysten/sui.js/bcs";

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
    const subAndPath = getSubdomainAndPath(url);
    if (subAndPath && subAndPath.subdomain) {
        const rpcUrl = getFullnodeUrl(NETWORK);
        const client = new SuiClient({ url: rpcUrl });

        let objectId = getObjectIdFromSuiNs(subAndPath.subdomain);
        if (!objectId) {
            objectId = subdomainToObjectId(subAndPath.subdomain);
        }
        if (objectId) {
            console.log("Object ID: ", objectId);
            console.log(
                "Base36 version of the object ID: ",
                b36.encode(fromHEX(objectId))
            );
            let pagePath = subAndPath.path ? subAndPath.path : "index.html";
            event.respondWith(fetchPage(client, objectId, pagePath));
            return;
        }
        event.respondWith(siteNotFound());
        return;
    }

    // Handle the case in which we are at the root
    if (url === scope || url === scope + "index.html") {
        // TODO: handle this better
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
    subdomain = subdomain.toLowerCase();
    let hexObjectId = toHEX(b36.decode(subdomain));
    let objectId = "0x" + hexObjectId;
    return isValidSuiObjectId(objectId) ? objectId : null;
}

type Path = {
    subdomain: string;
    path: string | null;
};

function getSubdomainAndPath(url: string): Path | null {
    // At the moment we only support one subdomain level.
    const REGEX = new RegExp(
        `^https://(?<subdomain>[A-Za-z0-9]+)\.${BASE_URL}/(?<path>[A-Za-z0-9/.-]*)`
    );
    const match = url.match(REGEX);
    if (match?.groups?.subdomain) {
        let path = null;
        if (match?.groups?.path) {
            path = removeFirstSlash(match.groups.path);
        }
        return { subdomain: match.groups.subdomain, path } as Path;
    }
    return null;
}

/** Remove the last forward-slash if present */
function removeFirstSlash(path: string): string {
    if (path[path.length - 1] === "/") {
        path = path.slice(0, -1);
    }
    return path;
}

// SuiNS-like functionality //
// Right now this just matches the subdomain to fixed strings. Should be replaced with a SuiNS lookup.

function getObjectIdFromSuiNs(subdomain: string): string | null {
    if (subdomain in SITE_NAMES) {
        return SITE_NAMES[subdomain];
    }
    return null;
}

// Types & encoding/decoding for the on-chain objects //

// Fetching objects from the full node //
type Blocksite = {
    updated: number | null;
    created: number;
    contents: Uint8Array;
    version: number;
    // TODO: Add fields for HTTP headers
};

type BlockPage = {
    name: string;
    created: number;
    updated: number | null;
    version: number;
    content_type: string;
    content_encoding: string;
    contents: Uint8Array;
    // TODO: Add fields for HTTP headers
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
    contents: VECTOR,
});

const FieldStruct = bcs.struct("Field", {
    id: UID,
    name: VECTOR,
    value: BlockPageStruct,
});

// Fectching & decompressing on-chain data //

/** Fetch the page */
async function fetchPage(
    client: SuiClient,
    objectId: string,
    path: string
): Promise<Response> {
    let blockPage = await client
        .getDynamicFieldObject({
            parentId: objectId,
            name: { type: "0x1::string::String", value: path },
        })
        .then((dynamicFields) => {
            return client.getObject({
                id: dynamicFields.data!.objectId,
                options: { showBcs: true },
            });
        })
        .then((pageData) => {
            return getPageFields(pageData.data!);
        });
    if (!blockPage.contents) {
        return siteNotFound();
    }
    let decompressed = await decompressData(
        blockPage.contents,
        blockPage.content_encoding
    );
    if (!decompressed) {
        return siteNotFound();
    }
    return new Response(decompressed, {
        headers: {
            "Content-Type": blockPage.content_type,
        },
    });
}

// Type definitions for BCS decoding //
function getPageFields(data: SuiObjectData): BlockPage | null {
    // Deserialize the bcs encoded struct
    if (data.bcs.dataType === "moveObject") {
        let blockpage = FieldStruct.parse(fromB64(data.bcs.bcsBytes));
        return blockpage.value;
    }
    return null;
}

async function decompressData(data: ArrayBuffer, contentEncoding: string) {
    if (contentEncoding === "plaintext") {
        return data;
    }
    // check that contentencoding is a valid CompressionFormat
    if (["gzip", "deflate", "deflate-raw"].includes(contentEncoding)) {
        let enc = contentEncoding as CompressionFormat;
        let blob = new Blob([data], { type: "application/gzip" });
        let stream = blob.stream().pipeThrough(new DecompressionStream(enc));
        let response = await new Response(stream).arrayBuffer().catch((e) => {
            console.error("DecompressionStream error", e);
        });
        if (response) return response;
    }
}

function siteNotFound(): Response {
    return new Response("<p>404 - Blocksite not found</p>", {
        status: 404,
        headers: {
            "Content-Type": "text/html",
        },
    });
}
