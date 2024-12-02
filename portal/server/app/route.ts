// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { getDomain, getSubdomainAndPath } from "@lib/domain_parsing";
import { redirectToAggregatorUrlResponse, redirectToPortalURLResponse } from "@lib/redirects";
import { getBlobIdLink, getObjectIdLink } from "@lib/links";
import { resolveAndFetchPage } from "@lib/page_fetching";
import { has } from '@vercel/edge-config';
import logger from "@lib/logger";
import * as Sentry from "@sentry/node";

function addLoggingArgsToSentry(args: { [key: string]: any }) {
    Object.entries(args).forEach(([key, value]) => {
        if (key !== "message") { // Skipping the 'message' key
            console.log(`${key}: ${value}`)
            Sentry.setTag(key, value);
        }
    });
}
logger.setErrorPredicate(args => {
    addLoggingArgsToSentry(args);
    Sentry.captureException(new Error(args.message ))
});
logger.setWarnPredicate(args => {
    addLoggingArgsToSentry(args);
    Sentry.addBreadcrumb({ message: args.message, data: args, level: 'warning' })
} );
logger.setInfoPredicate(args => {
    addLoggingArgsToSentry(args);
    Sentry.addBreadcrumb({ message: args.message, data: args, level: 'info'})
} );
logger.setDebugPredicate(args => {
    addLoggingArgsToSentry(args);
    Sentry.addBreadcrumb({ message: args.message, data: args, level: 'debug' })
});

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
    if (parsedUrl) {
        const subdomainIsBlocklisted = await has(parsedUrl.subdomain)
        if (subdomainIsBlocklisted) {
            logger.info({
                message: 'Attempt to access blocklisted subdomain',
                blocklistedSubdomain: parsedUrl.subdomain
            })
            return new Response(`Subdomain ${parsedUrl.subdomain} is blocklisted.`, { status: 403 });
        }
    }

    const portalDomain = getDomain(url, Number(portalDomainNameLength));
    const requestDomain = getDomain(url, Number(portalDomainNameLength));

    if (requestDomain == portalDomain && parsedUrl && parsedUrl.subdomain) {
        return await resolveAndFetchPage(parsedUrl, null);
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
