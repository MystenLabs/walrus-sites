// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { NextRequest } from "next/server";
import { getSubdomainAndPath } from "@lib/domain_parsing";
import { config } from "./configuration_loader";

/**
* Check if the request is for an HTML page.
* Used to avoid tracking requests for static assets like images, css, etc.
* i.e. Use this to only track page views.
* @param {NextRequest} request - The request object.
* @returns {Boolean} - True if the request is for an HTML page, false otherwise.
*/
export function isHtmlPage(request: NextRequest): Boolean{
    // This is to avoid tracking requests for static assets like images, css, etc.
    // Cuts down costs since we are tracking less events.
    const parsedUrl = getSubdomainAndPath(
        request.nextUrl,
        Number(config.portalDomainNameLength)
    );
    if (!parsedUrl?.path) {
    	throw new Error("No path found in parsed URL");
    }
    return parsedUrl?.path?.endsWith('.html')
}


/**
* Extract custom event properties from the request.
* @param {NextRequest} request - The request object.
* @returns {CustomEventProperties} - The extracted custom event properties.
*/
export function extractCustomEventProperties(request: NextRequest): CustomEventProperties {
	const parsedUrl = getSubdomainAndPath(
        request.nextUrl,
        Number(config.portalDomainNameLength)
    );

    return {
        originalUrl: request.headers.get("x-original-url") || "Unknown",
        subdomain: parsedUrl?.subdomain
    };
}

// As of this writing, vercel pro plan supports at most 2 custom event props.
type CustomEventProperties = {
    originalUrl: string;
    subdomain?: string;
};
