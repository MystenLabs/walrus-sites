// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { DomainDetails } from "@lib/types/index";
import { getDomain } from "@lib/domain_parsing";
import { blobAggregatorEndpoint } from "@lib/aggregator";
import { SuiClientTypes } from "@mysten/sui/client";
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
export function redirectToAggregatorUrlResponse(blobId: string, aggregatorUrl: string): Response {
    // Redirect to the walrus site for the specified domain and path
    const redirectUrl = blobAggregatorEndpoint(blobId, aggregatorUrl);
    logger.info("Redirecting to the Walrus Blob link", { redirectUrl: redirectUrl });
    return makeRedirectResponse(redirectUrl.href);
}

/**
 * Checks if the object has a redirect in its Display representation.
 *
 * IMPORTANT: this reads the *rendered* Display returned by gRPC
 * (`core.getObjects`), which only renders Display **v2** (the `@0xd` registry).
 * A legacy v1 Display comes back here as `undefined`, so the redirect silently
 * won't fire — the object's Display must have been migrated to v2. Verified: a
 * v1 Redirector rendered `undefined` over gRPC until migrated, then rendered the
 * field (JSON-RPC rendered both, which is why this was invisible pre-migration).
 */
export function checkRedirect(object: SuiClientTypes.Object<{ display: true }>): string | null {
    logger.info("Checking if the request should be redirected (existing Display object)", {
        objectId: object.objectId,
    });
    // Check if "walrus site address" is set in the rendered Display fields.
    const address = object.display?.output?.["walrus site address"];
    return typeof address === "string" ? address : null;
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
