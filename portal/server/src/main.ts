// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { getDomain, getSubdomainAndPath } from "@lib/domain_parsing";

import { siteNotFound } from "@lib/http/http_error_responses";
import blocklistChecker from "src/blocklist_checker";
import { config } from "src/configuration_loader";
import { urlFetcher } from "src/url_fetcher_factory";
import { Base36toHex } from "@lib/objectId_operations";
import { instrumentationFacade } from "@lib/instrumentation";
import { bringYourOwnDomainDoesNotSupportSubdomainsYet } from "@lib/http/http_error_responses";
import logger from "@lib/logger";

export default async function main(req: Request) {
    const url = new URL(req.url);
    logger.info("Processing new request", { url });

    const portalDomainNameLength = config.portalDomainNameLength;
    const parsedUrl = getSubdomainAndPath(url, Number(portalDomainNameLength));
    const portalDomain = getDomain(url, Number(portalDomainNameLength));
    const requestDomain = getDomain(url, Number(portalDomainNameLength));

    if (parsedUrl && !config.bringYourOwnDomain) {
        if (blocklistChecker && (await blocklistChecker.isBlocked(parsedUrl.subdomain))) {
            instrumentationFacade.bumpBlockedRequests();
            return siteNotFound();
        }

        if (requestDomain == portalDomain && parsedUrl.subdomain) {
            const res = await urlFetcher.resolveDomainAndFetchUrl(
                parsedUrl,
                null,
                blocklistChecker,
            );
            return res;
        }
    }

    const atBaseUrl = portalDomain == url.host.split(":")[0];
    if (atBaseUrl) {
        console.log("Serving the landing page from walrus...");
        // The landing page is an exception to the B36_DOMAIN_RESOLUTION_SUPPORT rule.
        // It will always resolve to an objectId.
        const response = await urlFetcher.resolveDomainAndFetchUrl(
            {
                subdomain: config.landingPageOidB36,
                path: parsedUrl?.path ?? "/index.html",
            },
            Base36toHex(config.landingPageOidB36),
            blocklistChecker,
        );
        return response;
    }

    if (config.bringYourOwnDomain) {
        return bringYourOwnDomainDoesNotSupportSubdomainsYet(parsedUrl?.subdomain!);
    }

    return new Response(`Resource at ${url} not found!`, { status: 404 });
}
