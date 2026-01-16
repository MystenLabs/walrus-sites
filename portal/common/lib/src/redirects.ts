// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { DomainDetails } from "@lib/types/index";
import { getDomain } from "@lib/domain_parsing";
import { blobAggregatorEndpoint } from "@lib/aggregator";
import { SuiObjectResponse } from "@mysten/sui/client";
import logger from "@lib/logger";

/**
 * Redirects to the portal URL.
 */
export function redirectToPortalURLResponse(
    scope: URL,
    path: DomainDetails,
    portalDomainNameLength?: number,
): Response {
    // Redirect to the walrus site for the specified domain and path
    const redirectUrl = getPortalUrl(path, scope.href, portalDomainNameLength);
    logger.info("Redirecting to the Walrus Site link", { path: path, redirectUrl: redirectUrl });
    return makeRedirectResponse(redirectUrl);
}

/**
 * Redirects to the aggregator URL.
 */
export function redirectToAggregatorUrlResponse(
    blobId: string,
    aggregatorUrl: string,
): Response {
    // Redirect to the walrus site for the specified domain and path
    const redirectUrl = blobAggregatorEndpoint(blobId, aggregatorUrl);
    logger.info("Redirecting to the Walrus Blob link", { redirectUrl: redirectUrl });
    return makeRedirectResponse(redirectUrl.href);
}

/**
 * Checks if the object has a redirect in its Display representation.
 */
export function checkRedirect(object: SuiObjectResponse): string | null {
    logger.info("Checking if the request should be redirected (existing Display object)", {
        objectId: object.data?.objectId,
    });
    if (object.data && object.data.display) {
        let display = object.data.display;
        // Check if "walrus site address" is set in the display field.
        if (display.data && display.data["walrus site address"]) {
            return display.data["walrus site address"];
        }
    }
    return null;
}

function makeRedirectResponse(url: string): Response {
    return new Response(null, {
        status: 302,
        headers: {
            Location: url,
        },
    });
}

/**
 * Returns the url for the Portal, given a subdomain and a path.
 */
function getPortalUrl(path: DomainDetails, scope: string, portalDomainNameLength?: number): string {
    const scopeUrl = new URL(scope) as URL;
    const portalDomain = getDomain(scopeUrl, portalDomainNameLength);
    let portString = "";
    if (scopeUrl.port) {
        portString = ":" + scopeUrl.port;
    }
    return scopeUrl.protocol + "//" + path.subdomain + "." + portalDomain + portString + path.path;
}
