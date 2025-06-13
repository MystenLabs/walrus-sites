// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import * as amplitude from "@amplitude/analytics-node";
import { generateHash, isHtmlPage } from "./utils";
import { config } from "./configuration_loader";
import logger from "@lib/logger";
import { getSubdomainAndPath } from "@lib/domain_parsing";
import { UAParser } from "ua-parser-js";

if (config.amplitudeApiKey) {
	amplitude.init(process.env.AMPLITUDE_API_KEY!,{
		// Events queued in memory will flush when number of events exceed upload threshold.
		// Default value is 30.
		flushQueueSize: 50,
		// Events queue will flush every certain milliseconds based on setting.
		// Default value is 10000 milliseconds.
		flushIntervalMillis: 20000, // Increase this to at least 10000 for production use.
	});
}

/**
* Sends a page view event to Amplitude.
* @param request - The incoming request to the portal.
*/
export async function sendToAmplitude(request: Request, originalUrl: URL): Promise<void> {
	if (!isHtmlPage(request)) {
		return;
	}
	if (!config.amplitudeApiKey) {
		logger.warn("Amplitude API key not found. Skipping tracking.");
		return;
	}
	try {
        // use originalUrl due to nextUrl would be http://localhost:3000/something behind a LB or reverse proxy
		const domainDetails = getSubdomainAndPath(originalUrl)
		let ua;
        let ua_header = request.headers.get("user-agent");
        if (!!ua_header) {
            try {
                ua = UAParser(ua_header);
            } catch (e) {
                console.warn("Could not parse user agent: ", e);
            }
        }
        let x_forwarded_for_ip = request.headers.get("x-forwarded-for")
        /// only use the first ip address in the x-forwarded-for header, as LB will add multiple ips that could lead to wrong geo location
        let ip = x_forwarded_for_ip ? x_forwarded_for_ip.split(",")[0] : request.headers.get("x-real-ip") ?? undefined;
		amplitude.track({
			os_name: ua?.os.name,
			os_version: ua?.os.version,
			device_id: generateDeviceId(ip, ua_header),
			device_manufacturer: ua?.device.vendor,
			platform: ua?.device.type,
	    	event_type: "page_view",
			language: request.headers.get("accept-language") ?? undefined,
			ip: ip,
			event_properties: {
				extra: {
					subdomain: domainDetails?.subdomain,
					originalUrl: request.headers.get('x-original-url'),
				},
			},
			user_agent: ua_header ?? undefined,
  	    })
	} catch (e) {
		console.warn("Amplitude could not track event: ", e);
	}
}

/**
* Generates a device ID based on the user agent string and ip addressj.
* @param ip - ip address.
* @param userAgent - device & browser details.
* @returns A hashed device ID.
*/
function generateDeviceId(ip: string | undefined, userAgent: string | null): string {
	const defaultDeviceId = "1234567890"
    let ip_str = ip ?? ""
    return generateHash(userAgent+ip_str || defaultDeviceId);
}
