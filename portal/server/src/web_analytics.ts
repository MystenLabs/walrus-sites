// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { track } from "@vercel/analytics/server";
import { isHtmlPage, extractCustomEventProperties } from "./utils";

/**
* Sends a page view event to Vercel Web Analytics.
* @param request - The incoming request to the portal.
*/
export async function sendToWebAnalytics(request: Request) {
    if (isHtmlPage(request)) {
        try {
        	const custom_event_properties = extractCustomEventProperties(request);
            await track('pageview', custom_event_properties)
        } catch (e) {
            console.warn("Could not track event: ", e);
        }
    }
}
