// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { serve, ServeOptions } from "bun";
import blocklist_healthcheck from "src/blocklist_healthcheck";
import CookieMonster from "src/cookie_monster";
import { genericError } from "@lib/http/http_error_responses";
import main from "src/main";
import { instrumentationFacade } from "@lib/instrumentation";
import { setupTapelog } from "custom_logger";

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
			const response = await main(request)
			CookieMonster.eatCookies(request, response)
			return response
		} catch (e) {
			instrumentationFacade.bumpGenericErrors();
			return genericError()
		}
	}
} as ServeOptions);
