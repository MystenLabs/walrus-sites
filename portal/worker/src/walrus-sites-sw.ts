// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { getDomain, getSubdomainAndPath } from "@lib/domain_parsing";
import { redirectToAggregatorUrlResponse, redirectToPortalURLResponse } from "@lib/redirects";
import { getBlobIdLink, getObjectIdLink } from "@lib/links";
import resolveWithCache from "./caching";
import { UrlFetcher } from "@lib/url_fetcher";
import { ResourceFetcher } from "@lib/resource";
import { RPCSelector } from "@lib/rpc_selector";
import { SuiNSResolver } from "@lib/suins";
import { WalrusSitesRouter } from "@lib/routing";

// This is to get TypeScript to recognize `clients` and `self` Default type of `self` is
// `WorkerGlobalScope & typeof globalThis` https://github.com/microsoft/TypeScript/issues/14877
declare var self: ServiceWorkerGlobalScope;
declare var clients: Clients;

const rpcUrlList = process.env.RPC_URL_LIST;
if (!rpcUrlList) {
    throw new Error("Missing RPC_URL_LIST environment variable");
}
const rpcSelector = new RPCSelector(rpcUrlList.split(','));
export const urlFetcher = new UrlFetcher(
    new ResourceFetcher(rpcSelector),
    new SuiNSResolver(rpcSelector),
    new WalrusSitesRouter(rpcSelector)
);

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

    // Check if the request is for a site.
    let portalDomainNameLengthString = process.env.PORTAL_DOMAIN_NAME_LENGTH;
    let portalDomainNameLength: number | undefined;
    if (portalDomainNameLengthString) {
        portalDomainNameLength = Number(portalDomainNameLengthString);
    }
    const objectIdPath = getObjectIdLink(urlString);
    if (objectIdPath) {
        event.respondWith(redirectToPortalURLResponse(scope, objectIdPath, portalDomainNameLength));
        return;
    }

    const walrusPath = getBlobIdLink(urlString);
    if (walrusPath) {
        event.respondWith(redirectToAggregatorUrlResponse(scope, walrusPath));
        return;
    }

    const parsedUrl = getSubdomainAndPath(url, Number(portalDomainNameLength));
    const portalDomain = getDomain(scope, Number(portalDomainNameLength));
    const requestDomain = getDomain(url, Number(portalDomainNameLength));

    console.log("Portal domain and request domain: ", portalDomain, requestDomain);
    console.log("Parsed URL: ", parsedUrl);

    if (requestDomain === portalDomain && parsedUrl && parsedUrl.subdomain) {

        // Fetches the page resources and handles the cache if it exists
        const handleFetchRequest = async (): Promise<Response> => {
            if ("caches" in self) {
                return await fetchFromCache();
            } else {
                console.warn("Cache API not available");
                return await urlFetcher.resolveDomainAndFetchUrl(parsedUrl, null);
            }
        };

        // Attempt to fetch from cache
        const fetchFromCache = async (): Promise<Response> => {
            console.log("Pre-fetching the sui object ID");
            return await resolveWithCache(parsedUrl, urlString);
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
