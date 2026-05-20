// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { serve, ServeOptions } from "bun";
import blocklist_healthcheck from "src/blocklist_healthcheck";
import CookieMonster from "src/cookie_monster";
import { genericError } from "@lib/http/http_error_responses";
import main from "src/main";
import { setupTapelog } from "custom_logger";
import logger, { formatErrorWithStack } from "@lib/logger";
import { AGGREGATOR_TIMEOUT_MS, QUILT_PATCH_ID_INTERNAL_HEADER } from "@lib/url_fetcher";
import { aggregatorExecutor } from "src/url_fetcher_factory";

const PORT = 3000;

// Headroom above the worst-case aggregator chain — accounts for RPC calls,
// hashing, and small per-attempt variance.
const IDLE_TIMEOUT_HEADROOM_S = 10;
// Upper bound on the computed idleTimeout so a misconfigured aggregator list
// (many URLs / high retries) can't grow the per-request budget unboundedly.
// Tune to fit under any upstream proxy or CDN request timeout sitting in front
// of the portal.
const IDLE_TIMEOUT_MAX_S = 100;

// Bun.serve closes the inbound socket after `idleTimeout` seconds. If that
// fires before the portal has returned a response, an upstream proxy may
// substitute its own (less helpful) error body. Size it from the aggregator
// retry budget so the chain always has room to complete and return our own
// aggregatorFail() response.
const idleTimeoutS = Math.min(
    Math.ceil(aggregatorExecutor.worstCaseDurationMs(AGGREGATOR_TIMEOUT_MS) / 1000) +
        IDLE_TIMEOUT_HEADROOM_S,
    IDLE_TIMEOUT_MAX_S,
);

console.log("Running Bun server at port", PORT, `(idleTimeout=${idleTimeoutS}s) ...`);
await setupTapelog();
serve({
    port: PORT,
    // Sized to the worst-case aggregator retry chain plus headroom so the
    // portal can return its own response instead of being cut mid-flight.
    idleTimeout: idleTimeoutS,
    // Special Walrus Sites routes.
    routes: {
        "/__wal__/*": async (req: Request) => {
            if (req.url.endsWith("/healthz")) {
                return await blocklist_healthcheck();
            }
            new Response("Not found!", {
                status: 404,
                statusText: "This special wal path does not exist.",
            });
        },
    },
    // The main flow of all other requests is here.
    async fetch(request: Request) {
        try {
            logger.context = Bun.randomUUIDv7(); // Track each request by adding a unique identifier.
            const response = await main(request);
            CookieMonster.eatCookies(request, response);
            return response;
        } catch (e) {
            logger.error("Unexpected uncaught exception during processing request", {
                error: formatErrorWithStack(e),
                // Get a subset of the request data to not include sensitive info.
                request: {
                    method: request.method,
                    url: new URL(request.url).pathname, // Excludes query params
                    headers: {
                        // Only log non-sensitive headers useful for debugging.
                        "user-agent": request.headers.get("user-agent"),
                        "content-type": request.headers.get("content-type"),
                        range: request.headers.get("range"),
                        accept: request.headers.get("accept"),
                        "accept-encoding": request.headers.get("accept-encoding"),
                        "if-none-match": request.headers.get("if-none-match"),
                        "if-modified-since": request.headers.get("if-modified-since"),
                        "cache-control": request.headers.get("cache-control"),
                        origin: request.headers.get("origin"),
                        [QUILT_PATCH_ID_INTERNAL_HEADER]: request.headers.get(
                            QUILT_PATCH_ID_INTERNAL_HEADER,
                        ),
                    },
                },
            });
            return genericError();
        }
    },
} as ServeOptions);
