// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { getDomain, getSubdomainAndPath } from "@lib/domain_parsing";
import { redirectToAggregatorUrlResponse, redirectToPortalURLResponse } from "@lib/redirects";
import { getBlobIdLink, getObjectIdLink } from "@lib/links";

import allowlistChecker from "src/allowlist_checker";
import { siteNotFound } from "@lib/http/http_error_responses";
import integrateLoggerWithSentry from "src/sentry_logger";
import blocklistChecker from "src/blocklist_checker";
import { config } from "src/configuration_loader";
import { standardUrlFetcher, premiumUrlFetcher } from "src/url_fetcher_factory";
import { NextRequest } from "next/server";

if (config.enableSentry) {
    // Only integrate Sentry on production.
    integrateLoggerWithSentry();
}

console.log(`Running Bun HTTP server at port ${3000}...`);
Bun.serve({
	async fetch(req: NextRequest) {
		const originalUrl = req.url;

		if (!originalUrl) {
			console.log('DEBUG', originalUrl)
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

			const urlFetcher = await allowlistChecker?.isAllowed(
				parsedUrl.subdomain ?? ''
			) ? premiumUrlFetcher : standardUrlFetcher;

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
})
