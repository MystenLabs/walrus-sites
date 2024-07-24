// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { getFullnodeUrl, SuiClient } from "@mysten/sui/client";
import { NETWORK } from "@lib/constants";
import template_404 from "@static/404-page.template.html";
import { getDomain, getSubdomainAndPath } from "@lib/domain_parsing";
import { DomainDetails, isResource } from "@lib/types/index";
import { redirectToAggregatorUrlResponse, redirectToPortalURLResponse } from "@lib/redirects";
import { aggregatorEndpoint } from "@lib/aggregator";
import { subdomainToObjectId, HEXtoBase36 } from "@lib/objectId_operations";
import { resolveSuiNsAddress, hardcodedSubdmains } from "@lib/suins";
import { getBlobIdLink, getObjectIdLink } from "@lib/links";
import { decompressData } from "@lib/decompress_data";
import { fetchResource } from "@lib/resource";

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

// Response errors returned.
// TODO: move to common lib. Need to resolve build error.

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
