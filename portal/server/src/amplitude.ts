// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import * as amplitude from "@amplitude/analytics-node";
import { generateHash, isHtmlPage } from "./utils";
import { config } from "./configuration_loader";
import logger from "@lib/logger";
import { NextRequest } from "next/server";

if (config.amplitudeApiKey) {
	amplitude.init(process.env.AMPLITUDE_API_KEY!,{
		// Events queued in memory will flush when number of events exceed upload threshold.
		// Default value is 30.
		flushQueueSize: 50,
		// Events queue will flush every certain milliseconds based on setting.
		// Default value is 10000 milliseconds.
		flushIntervalMillis: 5000, // TODO increase this to 20000 for production use.
	});
}

/**
* Sends a page view event to Amplitude.
* @param request - The incoming request to the portal.
*/
export async function sendToAmplitude(request: NextRequest): Promise<void> {
	if (!isHtmlPage(request)) {
		return;
	}
	if (!config.amplitudeApiKey) {
		logger.warn({ message: "Amplitude API key not found. Skipping tracking." });
		return;
	}
	try {
		amplitude.track({
			device_id: generateDeviceId(request.headers.get("user-agent")),
	    	event_type: "page_view",
  	    })
	} catch (e) {
		console.warn("Amplitude could not track event: ", e);
	}
}

/**
* Generates a device ID based on the user agent string.
* @param userAgent - device & browser details. Default is a random string.
* @returns A hashed device ID.
*/
function generateDeviceId(userAgent: string | null): string {
	const defaultDeviceId = "1234567890";
	return generateHash(userAgent || defaultDeviceId);
}
