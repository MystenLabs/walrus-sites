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
import { sendToWebAnalytics } from "src/web_analytics";
import { sendToAmplitude } from "src/amplitude";
import { Base36toHex } from "@lib/objectId_operations";

if (config.enableSentry) {
    // Only integrate Sentry on production.
    integrateLoggerWithSentry();
}

export default async function main(req: Request) {
	const url = new URL(req.url);

	// Send the page view event to either Amplitude or Vercel Web Analytics.
	if (config.amplitudeApiKey) {
		await sendToAmplitude(req, url);
	}
	if (config.enableVercelWebAnalytics) {
		await sendToWebAnalytics(req);
	}
	const portalDomainNameLength = config.portalDomainNameLength;
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
		// The landing page is an exception to the B36_DOMAIN_RESOLUTION_SUPPORT rule.
		// It will always resolve to an objectId.
		const urlFetcher = config.enableAllowlist ? premiumUrlFetcher : standardUrlFetcher;
		const response = await urlFetcher.resolveDomainAndFetchUrl(
			{
				subdomain: config.landingPageOidB36,
				path: parsedUrl?.path ?? "/index.html",
			},
			Base36toHex(config.landingPageOidB36),
			blocklistChecker
		);
		return response;
	}

	return new Response(`Resource at ${url} not found!`, { status: 404 });
}
