// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { NextRequest } from "next/server";
import { track } from "@vercel/analytics/server";
import { getSubdomainAndPath } from "@lib/domain_parsing";
import { config } from "src/configuration_loader";

export async function send_to_web_analytics(request: NextRequest) {
    // Track only when the request is for an HTML page.
    // This is to avoid tracking requests for static assets like images, css, etc.
    // Cuts down costs since we are tracking less events.
    const parsedUrl = getSubdomainAndPath(
        request.nextUrl,
        Number(config.portalDomainNameLength)
    );

    // Extract various details from the request
    const custom_event_properties = extract_custom_event_properties(request, parsedUrl?.subdomain)

    if (parsedUrl?.path?.endsWith('.html')) {
        try {
            await track('pageview', custom_event_properties)
        } catch (e) {
            console.warn("Could not track event: ", e);
        }
    }
}

function extract_custom_event_properties(request: NextRequest, subdomain?: string): CustomEventProperties {
    return {
        originalUrl: request.headers.get("x-original-url") || "Unknown",
        subdomain
    };
}

// As of this writing, vercel pro plan supports at most 2 custom event props.
type CustomEventProperties = {
    originalUrl: string;
    subdomain?: string;
};
