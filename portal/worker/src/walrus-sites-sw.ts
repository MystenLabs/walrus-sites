// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { getDomain, getSubdomainAndPath } from "@lib/domain_parsing";
import { redirectToAggregatorUrlResponse, redirectToPortalURLResponse } from "@lib/redirects";
import { getBlobIdLink, getObjectIdLink } from "@lib/links";
import resolveWithCache from "./caching";
import { resolveAndFetchPage } from "@lib/page_fetching";
import { FALLBACK_DEVNET_PORTAL } from "@lib/constants";

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

    if (requestDomain === portalDomain && parsedUrl && parsedUrl.subdomain) {
        // Fetches the page resources.
        const handleFetchRequest = async (): Promise<Response> => {
            try {
                if (!("caches" in self)) {
                    console.warn("Cache API not available");
                    return await resolveAndFetchPage(parsedUrl);
                }
                return await resolveWithCache(parsedUrl, urlString);
            } catch (error) {
                console.error("Error resolving the request:", error);
                return forwardToFallback();
            }
        };
        // If the original request fails, forward to the fallback portal.
        const forwardToFallback = async () => {
            return fetch(`${parsedUrl.subdomain}.${FALLBACK_DEVNET_PORTAL}`)
        };
        event.respondWith(
            handleFetchRequest()
                .then(response =>
                    response.status === 400 ? forwardToFallback() : response
                )
        );
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
    event.respondWith(fetch(event.request));
    return;
});
