// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { getFullnodeUrl, SuiClient, SuiObjectData } from "@mysten/sui/client";
import { fromB64 } from "@mysten/sui/utils";
import { RESOURCE_PATH_MOVE_TYPE, NETWORK, MAX_REDIRECT_DEPTH } from "@lib/constants";
import template_404 from "@static/404-page.template.html";
import { getDomain, getSubdomainAndPath } from "@lib/domain_parsing";
import { DomainDetails, Resource, isResource } from "@lib/types/index";
import { HttpStatusCodes } from "@lib/http_status_codes";
import { ResourceStruct, ResourcePathStruct, DynamicFieldStruct } from "@lib/bcs_data_parsing";
import { redirectToAggregatorUrlResponse, redirectToPortalURLResponse } from "@lib/redirects";
import { aggregatorEndpoint } from "@lib/aggregator";
import { subdomainToObjectId, HEXtoBase36 } from "@lib/objectId_operations";
import { resolveSuiNsAddress, hardcodedSubdmains } from "@lib/suins";
import { getBlobIdLink, getObjectIdLink } from "@lib/links";
import { checkRedirect } from "@lib/redirects";

// This is to get TypeScript to recognize `clients` and `self` Default type of `self` is
// `WorkerGlobalScope & typeof globalThis` https://github.com/microsoft/TypeScript/issues/14877
declare var self: ServiceWorkerGlobalScope;
declare var clients: Clients;

// Event listeners.

self.addEventListener("install", (_event) => {
    self.skipWaiting();
});

self.addEventListener("activate", (_event) => {
    clients.claim();
});

self.addEventListener("fetch", async (event) => {
    const urlString = event.request.url;
    const url = new URL(urlString);

    // Extract the range header from the request.
    const scopeString = self.registration.scope;
    const scope = new URL(scopeString);

    const objectIdPath = getObjectIdLink(urlString);
    if (objectIdPath) {
        event.respondWith(redirectToPortalURLResponse(scope, objectIdPath));
        return;
    }

    const walrusPath = getBlobIdLink(urlString);
    if (walrusPath) {
        event.respondWith(redirectToAggregatorUrlResponse(scope, walrusPath));
        return;
    }

    // Check if the request is for a site.
    const parsedUrl = getSubdomainAndPath(url);
    const portalDomain = getDomain(scope);
    const requestDomain = getDomain(url);

    console.log("Portal domain and request domain: ", portalDomain, requestDomain);
    console.log("Parsed URL: ", parsedUrl);

    if (requestDomain == portalDomain && parsedUrl && parsedUrl.subdomain) {
        console.log("fetching from the service worker");
        event.respondWith(resolveAndFetchPage(parsedUrl));
        return;
    }

    // Handle the case in which we are at the root `BASE_URL`
    if (urlString === scopeString || urlString === scopeString + "index.html") {
        console.log("serving the landing page");
        const newUrl = scopeString + "index-sw-enabled.html";
        event.respondWith(fetch(newUrl));
        return;
    }

    // Default case: Fetch all other sites from the web
    console.log("forwarding the request outside of the SW:", urlString);
    const response = await fetch(event.request);
    return response;
});

// Fectching & decompressing on-chain data.

/**
 * Resolves the subdomain to an object ID, and gets the corresponding resources.
 */
async function resolveAndFetchPage(parsedUrl: DomainDetails): Promise<Response> {
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
        console.log("Base36 version of the object ID: ", HEXtoBase36(objectId));
        return fetchPage(client, objectId, parsedUrl.path);
    }
    return noObjectIdFound();
}

/**
 * Fetches a page.
 */
async function fetchPage(client: SuiClient, objectId: string, path: string): Promise<Response> {
    const result = await fetchResource(client, objectId, path, new Set<string>);
    if (!isResource(result)) {
        const httpStatus = result as number;
        return new Response("Unable to fetch the site resource.", { status: httpStatus });
    }

    if (!result.blob_id) {
        if (path !== '/404.html') {
            return fetchPage(client, objectId, '/404.html');
        } else {
            return siteNotFound();
        }
    }

    console.log("Fetched Resource: ", result);
    const contents = await fetch(aggregatorEndpoint(result.blob_id));
    if (!contents.ok) {
        return siteNotFound();
    }

    // Deserialize the bcs encoded body and decompress.
    const body = new Uint8Array(await contents.arrayBuffer());
    const decompressed = await decompressData(body, result.content_encoding);
    if (!decompressed) {
        return siteNotFound();
    }
    console.log("Returning resource: ", result.path, result.blob_id, result.content_type);
    return new Response(decompressed, {
        headers: {
            "Content-Type": result.content_type,
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
 * This is useful to create many objects with an associated site (e.g., NFTs), without having to
 * repeat the same resources for each object, and allowing to keep some control over the site (for
 * example, the creator can still edit the site even if the NFT is owned by someone else).
 *
 * See the `checkRedirect` function for more details.
 * To prevent infinite loops, the recursion depth is of this function is capped to
 * `MAX_REDIRECT_DEPTH`.
 *
 * Infinite loops can also be prevented by checking if the resource has already been seen.
 * This is done by using the `seenResources` set.
 */
async function fetchResource(
    client: SuiClient,
    objectId: string,
    path: string,
    seenResources: Set<string>,
    depth: number = 0,
): Promise<Resource | HttpStatusCodes> {
    if (seenResources.has(objectId)) {
        return HttpStatusCodes.LOOP_DETECTED;
    } else if (depth >= MAX_REDIRECT_DEPTH) {
        return HttpStatusCodes.TOO_MANY_REDIRECTS;
    } else {
        seenResources.add(objectId);
    }

    let [redirectId, dynamicFields] = await Promise.all([
        checkRedirect(client, objectId),
        client.getDynamicFieldObject({
            parentId: objectId,
            name: { type: RESOURCE_PATH_MOVE_TYPE, value: path },
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
            return HttpStatusCodes.NOT_FOUND;
        }
        // Recurs increasing the recursion depth.
        return fetchResource(client, redirectId, path, seenResources, depth + 1);
    }

    console.log("Dynamic fields for ", objectId, dynamicFields);
    if (!dynamicFields.data) {
        console.log("No dynamic field found");
        return HttpStatusCodes.NOT_FOUND;
    }
    const pageData = await client.getObject({
        id: dynamicFields.data.objectId,
        options: { showBcs: true },
    });
    if (!pageData.data) {
        console.log("No page data found");
        return HttpStatusCodes.NOT_FOUND;
    }
    const siteResource = getResourceFields(pageData.data);
    if (!siteResource || !siteResource.blob_id) {
        return HttpStatusCodes.NOT_FOUND;
    }
    return siteResource;
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

// Response errors returned.

function siteNotFound(): Response {
    return Response404(
        "This page does not exist - the object ID is not a valid Walrus Site."
    );
}

function noObjectIdFound(): Response {
    return Response404("This page does not exist - no object ID could be found.");
}

function fullNodeFail(): Response {
    return Response404("Failed to contact the full node.");
}

function Response404(message: String): Response {
    console.log();
    return new Response(template_404.replace("${message}", message), {
        status: 404,
        headers: {
            "Content-Type": "text/html",
        },
    });
}
