// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { describe, it, expect, vi, beforeEach } from "vitest";

// Mock the instrumentation singleton so we can spy on counter bumps without
// binding to the Prometheus port.
vi.mock("@lib/instrumentation", () => ({
    instrumentationFacade: {
        bumpFullNodeFailRequests: vi.fn(),
        bumpAggregatorFailRequests: vi.fn(),
        bumpBlobUnavailableRequests: vi.fn(),
        bumpNoObjectIdFoundRequests: vi.fn(),
        bumpGenericErrors: vi.fn(),
        bumpSiteNotFoundRequests: vi.fn(),
        bumpBlockedRequests: vi.fn(),
        increaseRequestsMade: vi.fn(),
        recordResolveSuiNsAddressTime: vi.fn(),
        recordResolveDomainAndFetchUrlResponseTime: vi.fn(),
        recordAggregatorTime: vi.fn(),
        recordResourceNotFoundRequests: vi.fn(),
        recordHashMismatchRequests: vi.fn(),
        recordFetchRoutesAndRedirectsFieldObjectsTime: vi.fn(),
        recordRoutesAndRedirectsResolutionTime: vi.fn(),
        recordFullNodeFailRequests: vi.fn(),
    },
}));

import { UrlFetcher } from "@lib/url_fetcher";
import { ResourceFetcher } from "@lib/resource";
import { SuiNSResolver } from "@lib/suins";
import { WalrusSitesRouter } from "@lib/routing";
import { PriorityExecutor } from "@lib/priority_executor";
import { instrumentationFacade } from "@lib/instrumentation";
import { HttpStatusCodes } from "@lib/http/http_status_codes";

/**
 * The portal's `resolveObjectId` is supposed to bump
 * `ws_num_full_node_fail_counter` only when the Sui full node is genuinely
 * unreachable. In production we observe that the same counter is also being
 * bumped when a SuiNS subdomain is simply not registered: the FN responds
 * healthily with `error.code: "notExists"` for the dynamic field lookup, but
 * the @mysten/sui SDK turns that response into a thrown `ObjectError("Object
 * <id> does not exist")`. The portal's catch-all then treats this exception
 * the same as a real connectivity failure.
 *
 * These tests capture the desired behaviour:
 *   - A real connectivity failure (network error, timeout) → 503 + counter
 *     bumped.
 *   - A "name not registered" exception (`notExists` from the FN) → 404 (the
 *     same response we already return when `getNameRecord` resolves to null)
 *     and the counter is NOT bumped.
 *
 * The second test starts as RED on `main` and turns GREEN after the fix.
 */
describe("UrlFetcher.resolveObjectId — counter discrimination", () => {
    let suinsResolver: SuiNSResolver;
    let urlFetcher: UrlFetcher;

    beforeEach(() => {
        vi.clearAllMocks();

        suinsResolver = {
            hardcodedSubdomains: vi.fn().mockReturnValue(null),
            resolveSuiNsAddress: vi.fn(),
        } as unknown as SuiNSResolver;

        urlFetcher = new UrlFetcher(
            {} as ResourceFetcher,
            suinsResolver,
            {} as WalrusSitesRouter,
            new PriorityExecutor([{ url: "http://localhost:1", retries: 0, metric: 100 }]),
            // b36 resolution disabled so we always hit the SuiNS branch.
            false,
        );
    });

    /**
     * Builds the kind of `AggregateError` that bubbles up to `resolveObjectId`
     * after `RPCSelector.invokeWithFailover` exhausts retries. Each entry in
     * `errors[]` wraps the original underlying error as `cause` (see
     * priority_executor.ts).
     */
    function buildAggregateAfterFailover(causes: unknown[]): AggregateError {
        const wrapped = causes.map(
            (cause, i) =>
                new Error(`retry-same from http://localhost:1 (attempt ${i + 1})`, { cause }),
        );
        return new AggregateError(wrapped, "All URLs exhausted");
    }

    /** Mirrors @mysten/sui ObjectError shape (we don't import it because the
     * package doesn't re-export it from the public entry point). */
    class ObjectErrorLike extends Error {
        public code: string;
        constructor(code: string, message: string) {
            super(message);
            this.code = code;
            this.name = "ObjectError";
        }
    }

    it("returns 503 and bumps FN-fail counter on a real network failure", async () => {
        const networkErr = new TypeError("fetch failed");
        // simulate an undici-style cause chain
        (networkErr as Error & { cause?: unknown }).cause = Object.assign(
            new Error("ECONNREFUSED"),
            {
                code: "ECONNREFUSED",
            },
        );

        (suinsResolver.resolveSuiNsAddress as ReturnType<typeof vi.fn>).mockRejectedValue(
            buildAggregateAfterFailover([networkErr]),
        );

        const result = await urlFetcher.resolveObjectId({ subdomain: "anything", path: "/" });

        expect(result).toBeInstanceOf(Response);
        expect((result as Response).status).toBe(HttpStatusCodes.SERVICE_UNAVAILABLE);
        expect(instrumentationFacade.bumpFullNodeFailRequests).toHaveBeenCalledTimes(1);
    });

    it("returns 503 and bumps FN-fail counter on a request timeout", async () => {
        const timeoutErr = new Error("Request timed out");

        (suinsResolver.resolveSuiNsAddress as ReturnType<typeof vi.fn>).mockRejectedValue(
            buildAggregateAfterFailover([timeoutErr]),
        );

        const result = await urlFetcher.resolveObjectId({ subdomain: "anything", path: "/" });

        expect((result as Response).status).toBe(HttpStatusCodes.SERVICE_UNAVAILABLE);
        expect(instrumentationFacade.bumpFullNodeFailRequests).toHaveBeenCalledTimes(1);
    });

    it("returns 404 and does NOT bump FN-fail counter when the SuiNS name is not registered", async () => {
        // This is what @mysten/sui's getDynamicField → getObjects throws when
        // the FN cleanly answers "this object does not exist" (which is what
        // happens for any unregistered <name>.sui — the dynamic field's
        // address is queried directly).
        const objectErr = new ObjectErrorLike(
            "notExists",
            "Object 0x5f181e7e58e6f8f9c3a1c6e5d4b9e2a8d3f7c1b2a3e4d5c6b7a8f9e0d1c2b3a4 does not exist",
        );

        (suinsResolver.resolveSuiNsAddress as ReturnType<typeof vi.fn>).mockRejectedValue(
            buildAggregateAfterFailover([objectErr]),
        );

        const result = await urlFetcher.resolveObjectId({
            subdomain: "definitely-not-registered-xyz",
            path: "/",
        });

        expect((result as Response).status).toBe(HttpStatusCodes.NOT_FOUND);
        // The crux: this counter must NOT increment, otherwise our alert fires
        // for unregistered-name lookups even though the FN is healthy.
        expect(instrumentationFacade.bumpFullNodeFailRequests).not.toHaveBeenCalled();
    });

    it("returns 404 when the AggregateError mixes notExists with secondary-FN errors", async () => {
        // Real-world shape observed against testnet: the primary FN cleanly
        // returns notExists (the authoritative answer — name registration is
        // determinate on-chain state) but failover keeps going and the
        // secondary FNs (blastapi, suiet, etc.) often return 403/502 because
        // they're rate-limited or unauthenticated. We must not let those
        // unrelated failures inflate the FN-fail counter when we have proof
        // the name doesn't exist.
        const objectErr = new ObjectErrorLike("notExists", "Object 0xdeadbeef does not exist");
        const httpErr = Object.assign(new Error("Unexpected status code: 403"), {
            status: 403,
            statusText: "Forbidden",
        });
        const httpErr2 = Object.assign(new Error("Unexpected status code: 502"), {
            status: 502,
            statusText: "Bad Gateway",
        });

        (suinsResolver.resolveSuiNsAddress as ReturnType<typeof vi.fn>).mockRejectedValue(
            buildAggregateAfterFailover([objectErr, httpErr, httpErr2]),
        );

        const result = await urlFetcher.resolveObjectId({ subdomain: "mixed", path: "/" });

        expect((result as Response).status).toBe(HttpStatusCodes.NOT_FOUND);
        expect(instrumentationFacade.bumpFullNodeFailRequests).not.toHaveBeenCalled();
    });
});
