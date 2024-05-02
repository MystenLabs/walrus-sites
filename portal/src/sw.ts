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
import { SITE_NAMES, NETWORK } from "./constants";
import { bcs } from "@mysten/sui.js/bcs";
import template_404 from "../static/404-page.template.html";
import { collapseTextChangeRangesAcrossMultipleVersions } from "typescript";

// This is to get TypeScript to recognize `clients` and `self`
// Default type of `self` is `WorkerGlobalScope & typeof globalThis`
// https://github.com/microsoft/TypeScript/issues/14877
declare var self: ServiceWorkerGlobalScope;
declare var clients: Clients;

const RPC_URL_CACHE = "RPC_URL_CACHE";
const SHOW_NOTIFICATION_CACHE = "SHOW_NOTIFICATION_CACH";
var BASE36 = "0123456789abcdefghijklmnopqrstuvwxyz";
const b36 = baseX(BASE36);

// Event listeners //

self.addEventListener("install", (event) => {
    self.skipWaiting();
});

self.addEventListener("activate", (event) => {
    clients.claim();
});

/**
 * Listen to messages to set the full node rpc url
 * If the message does not contain a valid url, the full node rpc url is reset
 * to the default (sui.io)
 */
self.addEventListener("message", async (event) => {
    let data = event.data;
    if (data && data.type === "setFullNodeUrl") {
        console.log("Setting full node url: ", data.url);
        try {
            const url = new URL(data.url);
            await cacheRpcUrl(data.url);
        } catch {
            // The url is not valid, setting the default by deleting all entries
            await deleteAllCachedEntries(RPC_URL_CACHE);
        }
        confirmFullNodeUrl();
    }
    if (data && data.type === "getFullNodeUrl") {
        let url = await getFullNodeRpcUrl();
        console.log("Confirming full node url: ", url);
        confirmFullNodeUrl();
    }
    if (data && data.type === "notifyFullNode") {
        await cacheValue(data.show, SHOW_NOTIFICATION_CACHE);
    }
    if (data && data.type === "getNotificationSetting") {
        let show = await getNotificationSetting();
        console.log("Notification setting: ", show);
        const clients = await self.clients.matchAll();
        clients.forEach((client) => {
            client.postMessage({
                type: "notificationSetting",
                show: show,
            });
        });
    }
});

self.addEventListener("fetch", (event) => {
    const url = event.request.url;
    const scope = self.registration.scope;

    // Pass-through for the configuration page
    if (url === scope + "bscfg.html") {
        console.log("Passthrough");

        return fetch(event.request).then((response) => {
            return response;
        });
    }

    console.log("no passthrough");

    // Check if the request is for a blocksite
    const parsedUrl = getSubdomainAndPath(url);
    console.log(
        `Parsed URL ${url}\nsubdomain ${parsedUrl?.subdomain}\npath ${parsedUrl?.path}`
    );
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

/**
 * Remove the last forward-slash if present
 * Resources on chain are only stored as `/path/to/resource.extension`.
 */
function removeLastSlash(path: string): string {
    return path.endsWith("/") ? path.slice(0, -1) : path;
}

// SuiNS-like functionality //
// Right now this just matches the subdomain to fixed strings. Should be replaced with a SuiNS lookup.

/**
 * Resolve the subdomain to an object ID using SuiNS
 * The subdomain `example` will look up `example.sui` and return the object ID if found.
 */
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
    const rpcUrl = await getFullNodeRpcUrl();
    const client = new SuiClient({ url: rpcUrl });
    console.log("Loading with full node at: ", rpcUrl);
    await showFullNodeNotification(rpcUrl);
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

/**
 * Fetch the page
 */
async function fetchPage(
    client: SuiClient,
    objectId: string,
    path: string
): Promise<Response> {
    const blockResource = await fetchBlockResource(client, objectId, path);
    if (!blockResource || !blockResource.contents) {
        siteNotFound();
    }
    let contents = blockResource.contents;
    //  Either its a one-part page, or we need to fetch the other parts
    if (blockResource.parts >= 1) {
        // Fetch the other parts in parallel
        const otherParts = await Promise.all([
            ...partNames(path, blockResource.parts).map((part_name) =>
                fetchBlockResource(client, objectId, part_name)
            ),
        ]);
        // Merge all parts with the contents
        // TODO: better way?
        let contentsArray = [
            Array.from(contents),
            ...otherParts.map((part) => Array.from(part.contents)),
        ];
        contents = new Uint8Array(contentsArray.flat());
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
    path: string
): Promise<BlockResource | null> {
    const dynamicFields = await client.getDynamicFieldObject({
        parentId: objectId,
        name: { type: "0x1::string::String", value: path },
    });
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
    const blockPage = getPageFields(pageData.data);
    if (!blockPage || !blockPage.contents) {
        return null;
    }
    return blockPage;
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

/**
 * Get the url for the full node RPC
 * Either from the scriptURL of the service worker, or, if that is missing,
 * from the default RPC url
 */
async function getFullNodeRpcUrl(): Promise<string> {
    // const rpcUrl = new URLSearchParams(
    // new URL(self.registration.active.scriptURL).search
    // ).get("rpcUrl");
    let cachedUrl = await getCachedRpcUrl();
    return cachedUrl ? cachedUrl : getFullnodeUrl(NETWORK);
}

async function confirmFullNodeUrl() {
    const url = await getFullNodeRpcUrl();
    await showFullNodeNotification(url);
    const clients = await self.clients.matchAll();
    clients.forEach((client) => {
        client.postMessage({
            type: "fullNodeUrl",
            url: url,
        });
    });
}

async function showFullNodeNotification(url: string | null) {
    if (!url) {
        url = await getFullNodeRpcUrl();
    }
    if (await getNotificationSetting()) {
        self.registration.showNotification("Full Node Setting", {
            body: url,
        });
    }
}

// Cache operations //

/**
 * Store the RPC url in the cache
 * The cache should have only one entry at all times, in which the key only key
 * is the current RPC url.
 */
async function cacheRpcUrl(url: string) {
    await cacheValue(url, RPC_URL_CACHE);
}

/**
 * Delete all entries in the cache
 */
async function deleteAllCachedEntries(cache_name: string) {
    const cache = await caches.open(cache_name);
    const keys = await cache.keys();
    await Promise.all(keys.map((key) => cache.delete(key)));
}

async function cacheValue(value: string, cache_name: string) {
    await deleteAllCachedEntries(cache_name);
    const cache = await caches.open(cache_name);
    await cache.put(new Request(value), new Response());
}

async function getCachedValue(cache_name: string): Promise<string | null> {
    const cache = await caches.open(cache_name);
    const keys = await cache.keys();
    if (keys.length > 1) {
        console.log("ERROR: the cache contains more than 1 entry");
        return null;
    } else if (keys.length === 1) {
        return keys[0].url;
    } else {
        return null;
    }
}

/**
 * Get the RPC url currently stored in the cache
 */
async function getCachedRpcUrl(): Promise<string | null> {
    return getCachedValue(RPC_URL_CACHE);
}

async function getNotificationSetting(): Promise<boolean> {
    let value = await getCachedValue(SHOW_NOTIFICATION_CACHE);
    // The value is encoded as URL + "/true" or "/false". Recover the last part.
    if (value) {
        return value.split("/").pop() === "true";
    }
    return false;
}

// Responses //

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
