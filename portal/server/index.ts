// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { serve, ServeOptions } from "bun";
import blocklist_healthcheck from "src/blocklist_healthcheck";
import CookieMonster from "src/cookie_monster";
import { genericError } from "@lib/http/http_error_responses";
import main from "src/main";
import { setupTapelog } from "custom_logger";
import logger from "@lib/logger";
import { QUILT_PATCH_ID_INTERNAL_HEADER } from "@lib/url_fetcher";

const PORT = 3000;
console.log("Running Bun server at port", PORT, "...")
await setupTapelog()
serve({
	port: PORT,
	// Special Walrus Sites routes.
	routes: {
		"/__wal__/*": async (req: Request) => {
			if (req.url.endsWith("/healthz")) {
				return await blocklist_healthcheck()
			}
			new Response("Not found!", { status: 404, statusText: "This special wal path does not exist." })
		}
	},
	// The main flow of all other requests is here.
	async fetch(request: Request) {
		try {
			logger.context = Bun.randomUUIDv7(); // Track each request by adding a unique identifier.
			const response = await main(request)
			CookieMonster.eatCookies(request, response)
			return response
		} catch (e) {
			logger.error(
				"Unexpected uncaught exception during processing request",
				{
					error: e instanceof Error ? { name: e.name, message: e.message, stack: e.stack } : e,
					// Get a subset of the request data to not include sensitive info.
					request: {
					  method: request.method,
					  url: new URL(request.url).pathname, // Excludes query params
					  headers: {
						// Only log non-sensitive headers useful for debugging.
						'user-agent': request.headers.get('user-agent'),
						'content-type': request.headers.get('content-type'),
						'range': request.headers.get('range'),
						'accept': request.headers.get('accept'),
						'accept-encoding': request.headers.get('accept-encoding'),
						'if-none-match': request.headers.get('if-none-match'),
						'if-modified-since': request.headers.get('if-modified-since'),
						'cache-control': request.headers.get('cache-control'),
						'origin': request.headers.get('origin'),
						[QUILT_PATCH_ID_INTERNAL_HEADER]: request.headers.get(QUILT_PATCH_ID_INTERNAL_HEADER),
					  },
					},
				}
			);
			return genericError()
		}
	}
} as ServeOptions);
