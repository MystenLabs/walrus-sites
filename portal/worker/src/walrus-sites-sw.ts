// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { getDomain, getSubdomainAndPath } from "@lib/domain_parsing";
import { redirectToAggregatorUrlResponse, redirectToPortalURLResponse } from "@lib/redirects";
import { getBlobIdLink, getObjectIdLink } from "@lib/links";
import { resolveAndFetchPage } from "@lib/page_fetching";
import { SuiClient, getFullnodeUrl } from "@mysten/sui/client";
import { NETWORK } from "@lib/constants";
import { DomainDetails } from "@lib/types";

const CACHE_NAME = "walrus-sites-cache";
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
        await respondUsingCache(event, parsedUrl, urlString);
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
* Respond to the request using the cache API.
*/
async function respondUsingCache(event: FetchEvent, parsedUrl: DomainDetails, urlString: string) {
    event.respondWith((async () => {
        if (!('caches' in self)) {
            // When not being in a secure context, the Cache API is not available.
            console.warn('Cache API not available');
            return await resolveAndFetchPage(parsedUrl);
        }

        const cache = await caches.open(CACHE_NAME);
        const cachedResponse = await cache.match(urlString);
        const cacheWasFresh = !(await cleanExpiredCache(cachedResponse, urlString));
        let isCacheSameAsNetwork: boolean;
        try {
            if (cachedResponse && cacheWasFresh) {
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
}

/**
* Removes an entry of the cache, if that entry is expired.
*
* The expiration time is set by the `CACHE_EXPIRATION_TIME` constant.
* If the cached response is older than the expiration time, it's no longer
* "fresh" and it is removed from the cache.
*
* @param urlString the key of the cached entry to check
* @returns true if the cache entry was removed, false otherwise
*/
async function cleanExpiredCache(cachedResponse: Response, urlString: string): Promise<boolean> {
    const cache = await caches.open(CACHE_NAME);
    const now = Date.now();

    if (cachedResponse) { // Cache hit!
        const timestamp = parseInt(cachedResponse.headers.get("x-unix-time-cached") || "0");
        const hasExpired = now - timestamp > CACHE_EXPIRATION_TIME
        if (hasExpired) {
            await cache.delete(urlString);
            console.log('Removed expired cache entry:', urlString);
            return true;
        }
        console.log('Cache entry is still fresh:', urlString)
    }
    return false;
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
