// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { serve, ServeOptions } from "bun";
import blocklist_healthcheck from "src/blocklist_healthcheck";
import CookieMonster from "src/cookie_monster";
import { genericError } from "@lib/http/http_error_responses";
import main from "src/main";

const PORT = 3000;
console.log("Running Bun server at port", PORT, "...")
serve({
	port: PORT,
	// Special Walrus Sites routes.
	routes: {
		"/__wal__/*": async (req: Request) => {
			console.log("debug", req.url)
			if (req.url.endsWith("/healthz")) {
				return await blocklist_healthcheck()
			}
			new Response("Not found!", {status: 404, statusText: "This special wal path does not exist."})
 		},
		"/walrus-sites-sw.js": new Response(await Bun.file("./public/walrus-sites-sw.js").bytes(), {
			headers: {
				"Content-Type": "application/javascript",
			},
		}),
	},
	// The main flow of all other requests is here.
	async fetch(request: Request) {
		try {
			const response = await main(request)
			CookieMonster.eatCookies(request, response)
			return response
		} catch (e) {
			return genericError()
		}
	}
} as ServeOptions);
