// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { getDomain, getSubdomainAndPath } from "@lib/domain_parsing";
import { redirectToAggregatorUrlResponse, redirectToPortalURLResponse } from "@lib/redirects";
import { getBlobIdLink, getObjectIdLink } from "@lib/links";
import { resolveAndFetchPage } from "@lib/page_fetching";
import { SuiClient, getFullnodeUrl } from "@mysten/sui/client";
import { NETWORK } from "@lib/constants";

const cacheName = "walrus-sites-cache";
// TODO - move it to .env
const CACHE_EXPIRATION_TIME = 24 * 60 * 60 * 1000 // 24 hours in milliseconds

// This is to get TypeScript to recognize `clients` and `self` Default type of `self` is
// `WorkerGlobalScope & typeof globalThis` https://github.com/microsoft/TypeScript/issues/14877
declare var self: ServiceWorkerGlobalScope;
declare var clients: Clients;

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
        event.respondWith((async () => {
            // Clean the cache of expired entries
            await cleanExpiredCache();
            if (!('caches' in self)) {
                // When not being in a secure context, the Cache API is not available.
                console.warn('Cache API not available');
                return await resolveAndFetchPage(parsedUrl);
            }

            const cache = await caches.open(cacheName);
            const cachedResponse = await cache.match(urlString);
            let isCacheSameAsNetwork: boolean;
            try {
                if (cachedResponse) {
                    isCacheSameAsNetwork = await checkCachedVersionMatchesOnChain(cachedResponse);
                }
            } catch (e) {
                console.error("Error checking cache version against chain:", e);
            }
            if (cachedResponse && isCacheSameAsNetwork) {
                console.log("Cache hit!", urlString);
                return cachedResponse;
            } else {
                console.log("Cache miss!", urlString);
                const resolvedPage = await resolveAndFetchPage(parsedUrl);

                cache.put(urlString, resolvedPage.clone());
                return resolvedPage;
            }
        })());
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

/**
* Clean the cache of expired entries.
*
* Iterates over all the cache entries and removes the ones that have expired.
* The expiration time is set by the `CACHE_EXPIRATION_TIME` constant.
* The cache key contains the timestamp of the entry, which is used to determine
* if the entry has expired.
*
* The `CACHE_EXPIRATION_TIME` will not be large enough (usually max 24h)
* for the O(n) complexity to affect UX.
*/
async function cleanExpiredCache() {
    const cache = await caches.open(cacheName);
    const keys = await cache.keys();
    const now = Date.now();

    for (const urlString of keys) {
        const response = await cache.match(urlString);
        if (response) {
            const timestamp = parseInt(response.headers.get("x-unix-time-cached") || "0");
            if (now - timestamp > CACHE_EXPIRATION_TIME) {
                await cache.delete(urlString);
                console.log('Removed expired cache entry:', urlString.url);
            }
        }
    }
}

/**
* Check if the cached version of the Resource object matches the current on-chain version.
*
* @param cachedResponse the response to check the version of
* @returns true if the cached version matches the current version of the Resource object
*/
async function checkCachedVersionMatchesOnChain(cachedResponse: Response): Promise<boolean> {
    if (!cachedResponse){
        throw new Error("Cached response is null!");
    }
    const rpcUrl = getFullnodeUrl(NETWORK);
    const client = new SuiClient({ url: rpcUrl });
    const cachedVersion = cachedResponse.headers.get("x-resource-sui-object-version")
    const objectId = cachedResponse.headers.get("x-resource-sui-object-id");
    if (!cachedVersion || !objectId) {
        throw new Error("Cached response does not have the required headers");
    }
    const resourceObject = await client.getObject({id: objectId});
    if (!resourceObject.data) {
        throw new Error("Could not retrieve Resource object.");
    }
    console.log("Cached version: ", cachedVersion)
    console.log("Current version: ", resourceObject.data?.version)
    const currentObjectVersion = resourceObject.data?.version;
    return cachedVersion === currentObjectVersion;
}
