// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { NETWORK } from "@lib/constants";
import { SuiClient, getFullnodeUrl } from "@mysten/sui/client";
import { getDomain, getSubdomainAndPath } from "@lib/domain_parsing";
import { redirectToAggregatorUrlResponse, redirectToPortalURLResponse } from "@lib/redirects";
import { getBlobIdLink, getObjectIdLink } from "@lib/links";
import resolveWithCache from "./caching";
import { resolveAndFetchPage, resolveObjectId } from "@lib/page_fetching";
import { HttpStatusCodes } from "@lib/http/http_status_codes";

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
                return await fetchWithCacheSupport();
            } catch (error) {
                return handleFetchError(error);
            }
        };

        // Handle caching and fetching based on cache availability
        const fetchWithCacheSupport = async (): Promise<Response> => {
            if ("caches" in self) {
                return await fetchFromCache();
            } else {
                console.warn("Cache API not available");
                return await fetchDirectlyOrProxy();
            }
        };

        // Attempt to fetch from cache
        const fetchFromCache = async (): Promise<Response> => {
            const rpcUrl = getFullnodeUrl(NETWORK);
            const client = new SuiClient({ url: rpcUrl });
            console.log("Pre-fetching the sui object ID");
            const resolvedObjectId = await resolveObjectId(parsedUrl, client);
            if (typeof resolvedObjectId !== "string") {
                return resolvedObjectId;
            }
            const cachedResponse = await resolveWithCache(resolvedObjectId, parsedUrl, urlString);
            return cachedResponse.status === HttpStatusCodes.NOT_FOUND
                ? proxyFetch()
                : cachedResponse;
        };

        // Fetch directly and fallback if necessary
        const fetchDirectlyOrProxy = async (): Promise<Response> => {
            const response = await resolveAndFetchPage(parsedUrl, null);
            return response.status === HttpStatusCodes.NOT_FOUND ? proxyFetch() : response;
        };

        // Handle error during fetching
        const handleFetchError = (error: any): Promise<Response> => {
            console.error("Error resolving request:", error);
            console.log("Retrying from the fallback portal.");
            return proxyFetch();
        };

        // Fetch from the fallback URL
        const proxyFetch = async (): Promise<Response> => {
            const fallbackDomain = "blocksite.net";
            const fallbackUrl = `https://${parsedUrl.subdomain}.${fallbackDomain}${parsedUrl.path}`;
            console.info(`Falling back to the devnet portal! ${fallbackUrl}`);
            return fetch(fallbackUrl);
        };
        event.respondWith(handleFetchRequest());
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
