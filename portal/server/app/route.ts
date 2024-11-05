// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { getDomain, getSubdomainAndPath } from "@lib/domain_parsing";
import { redirectToAggregatorUrlResponse, redirectToPortalURLResponse } from "@lib/redirects";
import { getBlobIdLink, getObjectIdLink } from "@lib/links";
import { resolveAndFetchPage } from "@lib/page_fetching";
import { HttpStatusCodes } from "@lib/http/http_status_codes";

export async function GET(req: Request) {
    const originalUrl = req.headers.get("x-original-url");
    if (!originalUrl) {
        throw new Error("No original url found in request headers");
    }
    const url = new URL(originalUrl);

    const objectIdPath = getObjectIdLink(url.toString());
    if (objectIdPath) {
        console.log(`Redirecting to portal url response: ${url.toString()} from ${objectIdPath}`);
        return redirectToPortalURLResponse(url, objectIdPath);
    }
    const walrusPath: string | null = getBlobIdLink(url.toString());
    if (walrusPath) {
        console.log(`Redirecting to aggregator url response: ${req.url} from ${objectIdPath}`);
        return redirectToAggregatorUrlResponse(url, walrusPath);
    }

    // Check if the request is for a site.
    const parsedUrl = getSubdomainAndPath(url);
    const portalDomain = getDomain(url);
    const requestDomain = getDomain(url);

    if (requestDomain == portalDomain && parsedUrl && parsedUrl.subdomain) {
        const forwardToFallback = async () => {
            const subdomain = parsedUrl.subdomain;
            const fallbackDomain = process.env.FALLBACK_DEVNET_PORTAL;
            const fallbackUrl = `https://${subdomain}.${fallbackDomain}${parsedUrl.path}`;
            // We need to add the Accept-Encoding header to ensure that the fall back domain does
            // not reply with a compressed response.
            console.info(`Falling back to the devnet portal! ${fallbackUrl}`);
            return fetch(fallbackUrl, {
                headers: {
                    "Accept-Encoding": "identity",
                },
            });
        };

        try {
            const fetchPageResponse = await resolveAndFetchPage(parsedUrl, null);
            if (fetchPageResponse.status == HttpStatusCodes.NOT_FOUND) {
                return forwardToFallback();
            }
            return fetchPageResponse;
        } catch (error) {
            return forwardToFallback();
        }
    }

    const atBaseUrl = portalDomain == url.host.split(":")[0];
    if (atBaseUrl) {
        console.log("Serving the landing page from walrus...");
        const blobId = process.env.LANDING_PAGE_OID_B36!;
        const response = await resolveAndFetchPage(
            {
                subdomain: blobId,
                path: parsedUrl?.path ?? "/index.html",
            },
            null,
        );
        return response;
    }

    return new Response(`Resource at ${originalUrl} not found!`, { status: 404 });
}
