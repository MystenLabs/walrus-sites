// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { NextRequest } from "next/server";
import { track } from "@vercel/analytics/server";
import { isHtmlPage, extractCustomEventProperties } from "./utils";

export async function sendToWebAnalytics(request: NextRequest) {
    if (isHtmlPage(request)) {
        try {
        	const custom_event_properties = extractCustomEventProperties(request);
            await track('pageview', custom_event_properties)
        } catch (e) {
            console.warn("Could not track event: ", e);
        }
    }
}
