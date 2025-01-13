// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { NextRequest } from "next/server";
import { track } from "@vercel/analytics/server";

export async function send_to_web_analytics(request: NextRequest) {
    // Extract various details from the request
    const trackingData = extract_tracking_data(request)

    // Track only when the request is for an HTML page.
    // This is to avoid tracking requests for static assets like images, css, etc.
    // Cuts down costs since we are tracking less events.
    const originalUrl = request.headers.get("x-original-url");
    if (originalUrl?.endsWith('.html')) {
        await track('route-access', trackingData)
    }
}

function extract_tracking_data(request: NextRequest): CustomEventProperties {
    return {
        originalUrl: request.headers.get("x-original-url") || "Unknown",
    };
}

// As of this writing, vercel pro plan supports at most 2 custom event props.
type CustomEventProperties = {
    originalUrl: string;
};
