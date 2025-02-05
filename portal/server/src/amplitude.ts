// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import * as amplitude from "@amplitude/analytics-node";
import { generateHash, isHtmlPage } from "./utils";
import { config } from "./configuration_loader";
import logger from "@lib/logger";
import { NextRequest } from "next/server";
import { getSubdomainAndPath } from "@lib/domain_parsing";
import uaparser from "ua-parser-js";

if (config.amplitudeApiKey) {
	amplitude.init(process.env.AMPLITUDE_API_KEY!,{
		// Events queued in memory will flush when number of events exceed upload threshold.
		// Default value is 30.
		flushQueueSize: 50,
		// Events queue will flush every certain milliseconds based on setting.
		// Default value is 10000 milliseconds.
		flushIntervalMillis: 3000, // Increase this to at least 10000 for production use.
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
		const domainDetails = getSubdomainAndPath(request.nextUrl)
		let ua;
		try {
			ua = new uaparser.UAParser(request.headers.get("user-agent") ?? undefined);
		} catch (e) {
			console.warn("Could not parse user agent: ", e);
		}
		amplitude.track({
			os_name: ua?.getOS().name,
			os_version: ua?.getOS().version,
			device_id: generateDeviceId(request.headers.get("user-agent")),
	    	event_type: "page_view",
			region: request.geo?.region,
			country: request.geo?.country,
			location_lat: Number(request.geo?.latitude),
			location_lng: Number(request.geo?.longitude),
			language: request.headers.get("accept-language") ?? undefined,
			ip: request.headers.get("x-forwarded-for") ?? request.headers.get("x-real-ip") ?? undefined,
			extra: {
				walrus_site_subdomain: domainDetails?.subdomain,
			},
			user_agent: request.headers.get("user-agent") ?? undefined,
  	    })
	} catch (e) {
		console.warn("Amplitude could not track event: ", e);
	}
}

/**
* Generates a device ID based on the user agent string.
* @param userAgent - device & browser details.
* @returns A hashed device ID.
*/
function generateDeviceId(userAgent: string | null): string {
	const defaultDeviceId = "1234567890";
	return generateHash(userAgent || defaultDeviceId);
}
