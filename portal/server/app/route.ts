// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { getDomain, getSubdomainAndPath } from "@lib/domain_parsing";
import { redirectToAggregatorUrlResponse, redirectToPortalURLResponse } from "@lib/redirects";
import { getBlobIdLink, getObjectIdLink } from "@lib/links";

import { isAllowed } from "allowlist_checker";
import { UrlFetcher } from "@lib/url_fetcher";
import { ResourceFetcher } from "@lib/resource";
import { RPCSelector } from "@lib/rpc_selector";

import { siteNotFound } from "@lib/http/http_error_responses";
import integrateLoggerWithSentry from "sentry_logger";
import blocklistChecker from "custom_blocklist_checker";
import { SuiNSResolver } from "@lib/suins";
import { WalrusSitesRouter } from "@lib/routing";
import { config } from "configuration_loader";

if (config.enableSentry) {
    // Only integrate Sentry on production.
    integrateLoggerWithSentry();
}

const rpcSelector = new RPCSelector(config.rpcUrlList);
const urlFetcher = new UrlFetcher(
    new ResourceFetcher(rpcSelector),
    new SuiNSResolver(rpcSelector),
    new WalrusSitesRouter(rpcSelector)
);

export async function GET(req: Request) {
    const originalUrl = req.headers.get("x-original-url");
    if (!originalUrl) {
        throw new Error("No original url found in request headers");
    }
    const url = new URL(originalUrl);

    const objectIdPath = getObjectIdLink(url.toString());
    const portalDomainNameLength = config.portalDomainNameLength;
    if (objectIdPath) {
        console.log(`Redirecting to portal url response: ${url.toString()} from ${objectIdPath}`);
        return redirectToPortalURLResponse(url, objectIdPath, portalDomainNameLength);
    }
    const walrusPath: string | null = getBlobIdLink(url.toString());
    if (walrusPath) {
        console.log(`Redirecting to aggregator url response: ${req.url} from ${objectIdPath}`);
        return redirectToAggregatorUrlResponse(url, walrusPath);
    }

    const parsedUrl = getSubdomainAndPath(url, Number(portalDomainNameLength));
    const portalDomain = getDomain(url, Number(portalDomainNameLength));
    const requestDomain = getDomain(url, Number(portalDomainNameLength));

    if (parsedUrl) {
        if (blocklistChecker && await blocklistChecker.isBlocked(parsedUrl.subdomain)) {
            return siteNotFound();
        }

        const urlFetcher = await isAllowed(parsedUrl.subdomain ?? '') ? premiumUrlFetcher : standardUrlFetcher;
        if (requestDomain == portalDomain && parsedUrl.subdomain) {
            return await urlFetcher.resolveDomainAndFetchUrl(parsedUrl, null, blocklistChecker);
        }
    }

    const atBaseUrl = portalDomain == url.host.split(":")[0];
    if (atBaseUrl) {
        console.log("Serving the landing page from walrus...");
        // Always use the premium page fetcher for the landing page (when available).
        const urlFetcher = config.enableAllowlist ? premiumUrlFetcher : standardUrlFetcher;
        const response = await urlFetcher.resolveDomainAndFetchUrl(
            {
                subdomain: config.landingPageOidB36,
                path: parsedUrl?.path ?? "/index.html",
            },
            null,
            blocklistChecker
        );
        return response;
    }

    return new Response(`Resource at ${originalUrl} not found!`, { status: 404 });
}
