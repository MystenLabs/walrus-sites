import { getFullnodeUrl, SuiClient, SuiObjectData } from "@mysten/sui.js/client";
import * as baseX from "base-x";
import {
    fromB64,
    fromHEX,
    isValidSuiObjectId,
    isValidSuiAddress,
    toHEX,
} from "@mysten/sui.js/utils";
import {
    AGGREGATOR,
    SITE_PACKAGE,
    SITE_NAMES,
    NETWORK,
    MAX_REDIRECT_DEPTH,
} from "./constants";
import { bcs, BcsType } from "@mysten/bcs";
import template_404 from "../static/404-page.template.html";

// This is to get TypeScript to recognize `clients` and `self` Default type of `self` is
// `WorkerGlobalScope & typeof globalThis` https://github.com/microsoft/TypeScript/issues/14877
declare var self: ServiceWorkerGlobalScope;
declare var clients: Clients;

var BASE36 = "0123456789abcdefghijklmnopqrstuvwxyz";
const b36 = baseX(BASE36);

// Type definitions.

/**
 * The origin of the request, divied into subdomain and path.
 */
type Path = {
    subdomain: string;
    path: string;
};

/**
 * The metadata for a site resource, as stored on chain.
 */
type Resource = {
    path: string;
    content_type: string;
    content_encoding: string;
    blob_id: string;
};

// Structs for parsing BCS data.

const Address = bcs.bytes(32).transform({
    input: (id: string) => fromHEX(id),
    output: (id) => toHEX(id),
});

// Blob IDs are represented on chain as u256, but serialized in URLs as URL-safe Base64.
const BLOB_ID = bcs.u256().transform({
    input: (id: string) => id,
    output: (id) => base64UrlSafeEncode(bcs.u256().serialize(id).toBytes()),
});

const ResourcePathStruct = bcs.struct("ResourcePath", {
    path: bcs.string(),
});

const ResourceStruct = bcs.struct("Resource", {
    path: bcs.string(),
    content_type: bcs.string(),
    content_encoding: bcs.string(),
    blob_id: BLOB_ID,
});

function DynamicFieldStruct<K, V>(K: BcsType<K>, V: BcsType<V>) {
    return bcs.struct("DynamicFieldStruct<T>", {
        parentId: Address,
        name: K,
        value: V,
    });
}

// Event listeners.

self.addEventListener("install", (_event) => {
    self.skipWaiting();
});

self.addEventListener("activate", (_event) => {
    clients.claim();
});

self.addEventListener("fetch", async (event) => {
    const url = event.request.url;
    const scope = self.registration.scope;

    // Check if the request is for a site.
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

// Subdomain handling.

/**
 * Subdomain encoding and parsing.
 *
 * Use base36 instead of HEX to encode object ids in the subdomain, as the subdomain must be < 64
 * characters.  The encoding must be case insensitive.
 */
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

function getSubdomainAndPath(scope: string): Path | null {
    // At the moment we only support one subdomain level.
    const url = new URL(scope);
    const hostname = url.hostname.split(".");

    // TODO(giac): This should be changed to allow for SuiNS subdomains.
    if (hostname.length === 3 || (hostname.length === 2 && hostname[1] === "localhost")) {
        // Accept only one level of subdomain eg `subdomain.example.com` or `subdomain.localhost` in
        // case of local development.
        const path = url.pathname == "/" ? "/index.html" : removeLastSlash(url.pathname);
        return { subdomain: hostname[0], path } as Path;
    }
    return null;
}

/**
 * Removes the last forward-slash if present
 *
 * Resources on chain are stored as `/path/to/resource.extension` exclusively.
 */
function removeLastSlash(path: string): string {
    return path.endsWith("/") ? path.slice(0, -1) : path;
}

// SuiNS functionality.

/**
 * Resolves the subdomain to an object ID using SuiNS.
 *
 * The subdomain `example` will look up `example.sui` and return the object ID if found.
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

// Fectching & decompressing on-chain data.

/**
 * Resolves the subdomain to an object ID, and gets the corresponding resources.
 */
async function resolveAndFetchPage(parsedUrl: Path): Promise<Response> {
    const rpcUrl = getFullnodeUrl(NETWORK);
    const client = new SuiClient({ url: rpcUrl });

    let objectId = hardcodedSubdmains(parsedUrl.subdomain);
    if (!objectId) {
        // Try to convert the subdomain to an object ID NOTE: This effectively _disables_ any SuiNs
        // name that is the base36 encoding of an object ID (i.e., a 32-byte string). This is
        // desirable, prevents people from getting suins names that are the base36 encoding the
        // object ID of a target site (with the goal of hijacking non-suins queries)
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

/**
 * Fetches a page.
 */
async function fetchPage(client: SuiClient, objectId: string, path: string): Promise<Response> {
    const resource = await fetchResource(client, objectId, path);
    if (resource === null || !resource.blob_id) {
        return siteNotFound();
    }

    console.log("Fetched Resource: ", resource);
    const contents = await fetch(aggregatorEndpoint(resource.blob_id));
    if (!contents.ok) {
        return siteNotFound();
    }

    // Deserialize the bcs encoded body and decompress.
    const body = new Uint8Array(await contents.arrayBuffer());
    const decompressed = await decompressData(body, resource.content_encoding);
    if (!decompressed) {
        return siteNotFound();
    }
    return new Response(decompressed, {
        headers: {
            "Content-Type": resource.content_type,
        },
    });
}

/**
 * Fetches a resource of a site.
 *
 * This function is recursive, as it will follow the special redirect field if it is set. A site can
 * have a special redirect field that points to another site, where the resources to display the
 * site are found.
 *
 * This is usefult to create many objects with an associated site (e.g., NFTs), without having to
 * repeat the same resources for each object, and allowing to keep some control over the site (for
 * example, the creator can still edit the site even if the NFT is owned by someone else).
 * See the `specitalRedirectField` function for more details.
 *
 * To prevent infinite loops, the recursion depth is of this function is capped to
 * `MAX_REDIRECT_DEPTH`.
 */
async function fetchResource(
    client: SuiClient,
    objectId: string,
    path: string,
    depth: number = 0
): Promise<Resource | null> {
    if (depth > MAX_REDIRECT_DEPTH) {
        // TODO(giac): add return codes and return 508 "loop detected" or similar.
        return null;
    }

    let [redirectId, dynamicFields] = await Promise.all([
        checkRedirect(client, objectId),
        client.getDynamicFieldObject({
            parentId: objectId,
            name: { type: resourcePathMoveType(), value: path },
        }),
    ]);

    if (redirectId) {
        console.log("Redirect found");
        const redirectPage = await client.getObject({
            id: redirectId,
            options: { showBcs: true },
        });
        console.log("Redirect page: ", redirectPage);
        if (!redirectPage.data) {
            return null;
        }
        // Recurs increasing the recursion depth.
        return fetchResource(client, redirectId, path, depth + 1);
    }

    console.log("Dynamic fields for ", objectId, dynamicFields);
    if (!dynamicFields.data) {
        console.log("No dynamic field found");
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
    const blockPage = getResourceFields(pageData.data);
    if (!blockPage || !blockPage.blob_id) {
        return null;
    }
    return blockPage;
}

/**
 * The string representing the ResourcePath struct in the walrus_site package.
 */
function resourcePathMoveType(): string {
    return SITE_PACKAGE + "::site::ResourcePath";
}

/**
 * Checks if the object has a redirect in its Display representation.
 */
async function checkRedirect(client: SuiClient, objectId: string): Promise<string | null> {
    const object = await client.getObject({ id: objectId, options: { showDisplay: true } });
    if (object.data && object.data.display) {
        let display = object.data.display;
        // Check if "walrus site address" is set in the display field.
        if (display.data && display.data["walrus site address"]) {
            return display.data["walrus site address"];
        }
    }
    return null;
}

/**
 * Parses the resource information from the Sui object data response.
 */
function getResourceFields(data: SuiObjectData): Resource | null {
    // Deserialize the bcs encoded struct
    if (data.bcs && data.bcs.dataType === "moveObject") {
        const df = DynamicFieldStruct(ResourcePathStruct, ResourceStruct).parse(
            fromB64(data.bcs.bcsBytes)
        );
        return df.value;
    }
    return null;
}

/**
 * Parses the redirect information from the Sui object data response.
 */
// TODO(giac): Change to have the redirect field to be in the display field (#49).
function getRedirectField(data: SuiObjectData): string | null {
    // Deserialize the bcs encoded struct
    if (data.bcs && data.bcs.dataType === "moveObject") {
        const df = DynamicFieldStruct(bcs.string(), Address).parse(fromB64(data.bcs.bcsBytes));
        return df.value;
    }
    return null;
}

/**
 * Decompresses the contents of the buffer according to the content encoding.
 */
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

// Walrus-specific encoding.

/**
 * Returns the URL to fetch the blob of given ID from the aggregator/cache.
 */
function aggregatorEndpoint(blob_id: string): URL {
    return new URL(AGGREGATOR + "/v1/" + blob_id);
}

/**
 * Converts the given bytes to Base 64, and then converts it to URL-safe Base 64.
 *
 * See [wikipedia](https://en.wikipedia.org/wiki/Base64#URL_applications).
 */
function base64UrlSafeEncode(data: Uint8Array): string {
    let base64 = arrayBufferToBas64(data);
    // Use the URL-safe Base 64 encoding by removing padding and swapping characters.
    return base64.replaceAll("/", "_").replaceAll("+", "-").replaceAll("=", "");
}

function arrayBufferToBas64(bytes: Uint8Array): string {
    // Convert each byte in the array to the correct character
    const binaryString = Array.from(bytes, (byte) => String.fromCharCode(byte)).join("");
    // Encode the binary string to base64 using btoa
    return btoa(binaryString);
}

// Response errors returned.

function siteNotFound(): Response {
    return Response404(
        "The URL provided points to an object ID, but the object does not seem to be a Walrus site."
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
