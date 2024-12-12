// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { getDomain, getSubdomainAndPath } from "@lib/domain_parsing";
import { redirectToAggregatorUrlResponse, redirectToPortalURLResponse } from "@lib/redirects";
import { getBlobIdLink, getObjectIdLink } from "@lib/links";
import { PageFetcher } from "@lib/page_fetching";
import { ResourceFetcher } from "@lib/resource";
import { siteNotFound } from "@lib/http/http_error_responses";
import integrateLoggerWithSentry from "sentry_logger";
import blocklistChecker from "custom_blocklist_checker";

if (process.env.ENABLE_SENTRY === "true") {
    // Only integrate Sentry on production.
    integrateLoggerWithSentry();
}

const pageFetcher = new PageFetcher(
    new ResourceFetcher()
);

export async function GET(req: Request) {
    const originalUrl = req.headers.get("x-original-url");
    if (!originalUrl) {
        throw new Error("No original url found in request headers");
    }
    const url = new URL(originalUrl);

    // Check if the request is for a site.
    let portalDomainNameLengthString = process.env.PORTAL_DOMAIN_NAME_LENGTH;
    let portalDomainNameLength: number | undefined;
    if (process.env.PORTAL_DOMAIN_NAME_LENGTH) {
        portalDomainNameLength = Number(portalDomainNameLengthString);
    }

    const objectIdPath = getObjectIdLink(url.toString());
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

        if (requestDomain == portalDomain && parsedUrl.subdomain) {
            return await pageFetcher.resolveAndFetchPage(parsedUrl, null, blocklistChecker);
        }
    }

    const atBaseUrl = portalDomain == url.host.split(":")[0];
    if (atBaseUrl) {
        console.log("Serving the landing page from walrus...");
        const blobId = process.env.LANDING_PAGE_OID_B36!;
        const response = await pageFetcher.resolveAndFetchPage(
            {
                subdomain: blobId,
                path: parsedUrl?.path ?? "/index.html",
            },
            null,
            blocklistChecker
        );
        return response;
    }

    return new Response(`Resource at ${originalUrl} not found!`, { status: 404 });
}
