// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { serve, ServeOptions } from "bun";
import blocklist_healthcheck from "src/blocklist_healthcheck";
import main from "src/main";

const PORT = 3000;
console.log("Running Bun server at port", PORT, "...")
serve({
	port: PORT,
	// Special Walrus Sites routes.
	routes: {
		"/api/healthz": await blocklist_healthcheck(),
		"/walrus-sites-sw.js": new Response(await Bun.file("./public/walrus-sites-sw.js").bytes(), {
			headers: {
				"Content-Type": "application/javascript",
			},
		}),
	},
	// The main flow of all other requests is here.
	fetch(request: Request) {
		return main(request)
	}
} as ServeOptions);
