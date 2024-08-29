// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { getDomain, getSubdomainAndPath } from "@lib/domain_parsing";
import { redirectToAggregatorUrlResponse, redirectToPortalURLResponse } from "@lib/redirects";
import { getBlobIdLink, getObjectIdLink } from "@lib/links";
import resolveWithCache from './caching'
import { resolveAndFetchPage } from "@lib/page_fetching";

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
        if (!("caches" in self)) {
            // When not being in a secure context, the Cache API is not available.
            console.warn("Cache API not available");
            const response = resolveAndFetchPage(parsedUrl)
            event.respondWith(response);
            return
        }
        const response = resolveWithCache(parsedUrl, urlString)
        return event.respondWith(response);
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
