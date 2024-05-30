import { getFullnodeUrl, SuiClient, SuiObjectData } from "@mysten/sui.js/client";
import * as baseX from "base-x";
import {
    fromB64,
    fromHEX,
    isValidSuiObjectId,
    isValidSuiAddress,
    toHEX,
} from "@mysten/sui.js/utils";
import { AGGREGATOR, BLOCKSITE_OID, SITE_NAMES, NETWORK } from "./constants";
import { bcs } from "@mysten/sui.js/bcs";
import template_404 from "../static/404-page.template.html";

// This is to get TypeScript to recognize `clients` and `self` Default type of `self` is
// `WorkerGlobalScope & typeof globalThis` https://github.com/microsoft/TypeScript/issues/14877
declare var self: ServiceWorkerGlobalScope;
declare var clients: Clients;

var BASE36 = "0123456789abcdefghijklmnopqrstuvwxyz";
const b36 = baseX(BASE36);

self.addEventListener("install", (_event) => {
    self.skipWaiting();
});

self.addEventListener("activate", (_event) => {
    clients.claim();
});

self.addEventListener("fetch", async (event) => {
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
    const response = await fetch(event.request);
    return response;
});

// Subdomain encoding & parsing.
//
// Use base36 instead of HEX to encode object ids in the subdomain, as the subdomain must be < 64
// characters.  The encoding must be case insensitive.

function subdomainToObjectId(subdomain: string): string | null {
    const objectId = "0x" + toHEX(b36.decode(subdomain.toLowerCase()));
    console.log(
        "obtained object id: ",
        objectId,
        isValidSuiObjectId(objectId),
        isValidSuiAddress(objectId)
    );
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

    if (hostname.length === 3 || (hostname.length === 2 && hostname[1] === "localhost")) {
        // Accept only one level of subdomain eg `subdomain.example.com` or `subdomain.localhost` in
        // case of local development.
        // TODO: This should be changed to allow for SuiNS subdomains.
        const path = url.pathname == "/" ? "/index.html" : removeLastSlash(url.pathname);
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

// SuiNS-like functionality.

/** Resolve the subdomain to an object ID using SuiNS The subdomain
 * `example` will look up `example.sui` and return the object ID if
 * found.
 */
async function resolveSuiNsAddress(client: SuiClient, subdomain: string): Promise<string | null> {
    const suiObjectId: string = await client.call("suix_resolveNameServiceAddress", [
        subdomain + ".sui",
    ]);
    console.log("resolved suins name: ", subdomain, suiObjectId);
    return suiObjectId ? suiObjectId : null;
}

function hardcodedSubdmains(subdomain: string): string | null {
    if (subdomain in SITE_NAMES) {
        return SITE_NAMES[subdomain];
    }
    return null;
}

// Types & encoding/decoding for the on-chain objects.

// Fetching objects from the full node.
type BlockResource = {
    path: string;
    content_type: string;
    content_encoding: string;
    blob_id: string;
};

// Define UID as a 32-byte array, then add a transform to/from hex strings.
const UID = bcs.fixedArray(32, bcs.u8()).transform({
    input: (id: string) => fromHEX(id),
    output: (id) => toHEX(Uint8Array.from(id)),
});

const VECTOR = bcs.vector(bcs.u8()).transform({
    input: (contents: Uint8Array) => Array.from(contents),
    output: (contents) => Uint8Array.from(contents),
});

const BLOB_ID = bcs.u256().transform({
    input: (id: string) => id,
    output: (id) => base64UrlSafeEncode(bcs.u256().serialize(id).toBytes()),
});

const BlockPageStruct = bcs.struct("BlockPage", {
    path: bcs.string(),
    content_type: bcs.string(),
    content_encoding: bcs.string(),
    blob_id: BLOB_ID,
});

const FieldStruct = bcs.struct("Field", {
    id: UID,
    name: VECTOR,
    value: BlockPageStruct,
});

const FieldStructRedirect = bcs.struct("Field", {
    id: UID,
    name: VECTOR,
    value: UID,
});

// Fectching & decompressing on-chain data.

async function resolveAndFetchPage(parsedUrl: Path): Promise<Response> {
    const rpcUrl = getFullnodeUrl(NETWORK);
    const client = new SuiClient({ url: rpcUrl });

    let objectId = hardcodedSubdmains(parsedUrl.subdomain);
    if (!objectId) {
        // Try to convert the subdomain to an object ID NOTE: This effectively _disables_ any SuiNs
        // name that is the base36 encoding of an object ID (i.e., a 32-byte string). This is
        // desirable, prevents people from getting suins names that are the base36 encoding the
        // object ID of a target blocksite (with the goal of hijacking non-suins queries)
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
        console.log("Base36 version of the object ID: ", b36.encode(fromHEX(objectId)));
        return fetchPage(client, objectId, parsedUrl.path);
    }
    return noObjectIdFound();
}

/** Fetch the page */
async function fetchPage(client: SuiClient, objectId: string, path: string): Promise<Response> {
    const blockResource = await fetchBlockResource(client, objectId, path);
    if (blockResource == null || !blockResource.blob_id) {
        siteNotFound();
    }

    console.log("Fetched Resource: ", blockResource);
    const contents = await fetch(aggregatorEndpoint(blockResource.blob_id));
    if (!contents.ok) {
        return siteNotFound();
    }

    // Deserialize the bcs encoded body and decompress.
    const body = new Uint8Array(await contents.arrayBuffer());

    console.log("body: ", body);
    const decompressed = await decompressData(body, blockResource.content_encoding);
    if (!decompressed) {
        return siteNotFound();
    }
    return new Response(decompressed, {
        headers: {
            "Content-Type": blockResource.content_type,
        },
    });
}

function specialRedirectField(): string {
    return BLOCKSITE_OID + "-site";
}

async function fetchBlockResource(
    client: SuiClient,
    objectId: string,
    path: string
): Promise<BlockResource | null> {
    // Also check the special site field
    const siteFieldPromise = client.getDynamicFieldObject({
        parentId: objectId,
        name: { type: "0x1::string::String", value: specialRedirectField() },
    });
    const dynamicFieldsPromise = client.getDynamicFieldObject({
        parentId: objectId,
        name: { type: "0x1::string::String", value: path },
    });
    let [siteField, dynamicFields] = await Promise.all([siteFieldPromise, dynamicFieldsPromise]);

    if (siteField.data) {
        console.log("Special redirect field found");
        const redirectPage = await client.getObject({
            id: siteField.data.objectId,
            options: { showBcs: true },
        });
        console.log("Redirect page: ", redirectPage);
        if (!redirectPage.data) {
            return null;
        }
        const redirectObjectId = getRedirectField(redirectPage.data);
        return fetchBlockResource(client, redirectObjectId, path);
    }

    console.log("Dynamic fields: ", dynamicFields);
    if (!dynamicFields.data) {
        console.log("No dynamic field found");
        console.log(dynamicFields);
        return null;
    }
    const pageData = await client.getObject({
        id: dynamicFields.data.objectId,
        options: { showBcs: true },
    });
    if (!pageData.data) {
        console.log("No page data found");
        return null;
    }
    const blockPage = getPageFields(pageData.data);
    if (!blockPage || !blockPage.blob_id) {
        return null;
    }
    return blockPage;
}

// Type definitions for BCS decoding.

function getPageFields(data: SuiObjectData): BlockResource | null {
    // Deserialize the bcs encoded struct
    if (data.bcs && data.bcs.dataType === "moveObject") {
        const blockpage = FieldStruct.parse(fromB64(data.bcs.bcsBytes));
        return blockpage.value;
    }
    return null;
}

function getRedirectField(data: SuiObjectData): string | null {
    // Deserialize the bcs encoded struct
    if (data.bcs && data.bcs.dataType === "moveObject") {
        const blockpage = FieldStructRedirect.parse(fromB64(data.bcs.bcsBytes));
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

function Response404(message: String): Response {
    console.log();
    return new Response(
        // TODO: better way for this?
        template_404.replace("${message}", message),
        {
            status: 404,
            headers: {
                "Content-Type": "text/html",
            },
        }
    );
}

function aggregatorEndpoint(blob_id: string): URL {
    return new URL(AGGREGATOR + "/v1/" + blob_id);
}

function base64UrlSafeEncode(data: Uint8Array): string {
    let base64 = arrayBufferToBas64(data);
    // Use the URL-safe base 64 encoding by removing padding and swapping characters.
    return base64.replaceAll("/", "_").replaceAll("+", "-").replaceAll("=", "");
}

function arrayBufferToBas64(bytes: Uint8Array): string {
    // Convert each byte in the array to the correct character
    const binaryString = Array.from(bytes, (byte) => String.fromCharCode(byte)).join("");
    // Encode the binary string to base64 using btoa
    return btoa(binaryString);
}
