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
 * Tests that `resolveObjectId` only bumps `ws_num_full_node_fail_counter`
 * for genuine FN connectivity failures.
 *
 * Unregistered SuiNS names are now handled at the RPCSelector level
 * (getNameRecord returns null), so they never reach the catch block here.
 * See rpc_selector_get_name_record.test.ts for the notExists tests.
 */
describe("UrlFetcher.resolveObjectId — FN-fail counter", () => {
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

    it("returns 503 and bumps FN-fail counter on a real network failure", async () => {
        (suinsResolver.resolveSuiNsAddress as ReturnType<typeof vi.fn>).mockRejectedValue(
            new TypeError("fetch failed"),
        );

        const result = await urlFetcher.resolveObjectId({ subdomain: "anything", path: "/" });

        expect(result).toBeInstanceOf(Response);
        expect((result as Response).status).toBe(HttpStatusCodes.SERVICE_UNAVAILABLE);
        expect(instrumentationFacade.bumpFullNodeFailRequests).toHaveBeenCalledTimes(1);
    });

    it("returns 503 and bumps FN-fail counter on a request timeout", async () => {
        (suinsResolver.resolveSuiNsAddress as ReturnType<typeof vi.fn>).mockRejectedValue(
            new Error("Request timed out"),
        );

        const result = await urlFetcher.resolveObjectId({ subdomain: "anything", path: "/" });

        expect((result as Response).status).toBe(HttpStatusCodes.SERVICE_UNAVAILABLE);
        expect(instrumentationFacade.bumpFullNodeFailRequests).toHaveBeenCalledTimes(1);
    });

    it("returns 404 and does NOT bump FN-fail counter when SuiNS name is not registered", async () => {
        // After the fix, getNameRecord returns null for unregistered names,
        // so resolveSuiNsAddress returns null — no throw, no catch block.
        (suinsResolver.resolveSuiNsAddress as ReturnType<typeof vi.fn>).mockResolvedValue(null);

        const result = await urlFetcher.resolveObjectId({
            subdomain: "definitely-not-registered-xyz",
            path: "/",
        });

        expect((result as Response).status).toBe(HttpStatusCodes.NOT_FOUND);
        expect(instrumentationFacade.bumpFullNodeFailRequests).not.toHaveBeenCalled();
    });
});
