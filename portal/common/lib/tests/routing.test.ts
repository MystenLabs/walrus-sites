// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { WalrusSitesRouter } from "@lib/routing";
import { test, expect, describe, vi, beforeEach } from "vitest";
import { RPCSelector } from "@lib/rpc_selector";
import { UrlFetcher, FetchUrlFailReason } from "@lib/url_fetcher";
import type { FetchUrlResult } from "@lib/url_fetcher";
import { ResourceFetcher } from "@lib/resource";
import { SuiNSResolver } from "@lib/suins";

const snakeSiteObjectId = "0x7a95e4be3948415b852fb287d455166a276d7a52f1a567b4a26b6b5e9c753158";
const rpcSelector = new RPCSelector(process.env.RPC_URL_LIST!.split(","), "testnet");
const wsRouter = new WalrusSitesRouter(rpcSelector);
const aggregatorUrl = process.env.AGGREGATOR_URL!;
const sitePackage = process.env.SITE_PACKAGE!;

test.skip("getRoutes", async () => {
    // TODO: when you make sure get_routes fetches
    // the Routes dynamic field, mock the request.
    const routes = await wsRouter.getRoutes(snakeSiteObjectId);
    console.log(routes);
});

const routesExample = {
    routes_list: new Map<string, string>([
        ["/*", "/default.html"],
        ["/somewhere/else", "/else.jpeg"],
        ["/somewhere/else/*", "/star-else.gif"],
        ["/path/to/*", "/somewhere.html"],
    ]),
};

const testCases = [
    ["/path/to/somewhere/", "/somewhere.html"],
    ["/somewhere/else", "/else.jpeg"],
    ["/", "/default.html"],
    ["/somewhere", "/default.html"],
    ["/somewhere/else/star", "/star-else.gif"],
    ["/somewhere/else/", "/star-else.gif"],
];

testCases.forEach(([requestPath, expected]) => {
    test(`matchPathToRoute: "${requestPath}" -> "${expected}"`, () => {
        const match = wsRouter.matchPathToRoute(requestPath, routesExample);
        expect(match).toEqual(expected);
    });
});

// Test in the case there are no routes.
const emptyRoutes = { routes_list: new Map<string, string>() };

testCases.forEach(([requestPath, _]) => {
    test(`matchPathToRoute: empty routes for "${requestPath}"`, () => {
        const match = wsRouter.matchPathToRoute(requestPath, emptyRoutes);
        expect(match).toEqual(undefined);
    });
});

describe("routing tests", () => {
    beforeEach(() => {
        vi.clearAllMocks();
    });

    test("should check routes before 404.html", async () => {
        const urlFetcher = new UrlFetcher(
            new ResourceFetcher(rpcSelector, sitePackage),
            new SuiNSResolver(rpcSelector),
            wsRouter,
            aggregatorUrl,
            true,
        );

        const fetchUrlSpy = vi.spyOn(urlFetcher, "fetchUrl");
        // Mock the fetchUrl method to return FetchUrlResult
        fetchUrlSpy.mockImplementation(
            async (objectId: string, path: string): Promise<FetchUrlResult> => {
                switch (path) {
                    case "/test.html":
                        return {
                            kind: "ok",
                            response: new Response("test.html content", { status: 200 }),
                        };
                    case "/404.html":
                        return {
                            kind: "ok",
                            response: new Response("404 page content", { status: 200 }),
                        };
                    default:
                        return {
                            kind: "notFound",
                            reason: FetchUrlFailReason.ResourceNotFound,
                        };
                }
            },
        );

        const getRoutesSpy = vi.spyOn(wsRouter, "getRoutes");
        // Mock the getRoutes method to return a test.html route
        getRoutesSpy.mockImplementation(async () => {
            return {
                routes_list: new Map([["/test", "/test.html"]]),
            };
        });

        const siteObjectId = "0x0977d45a9adb8af8405c0698b0e049de05f8c89da75ca16ac6a6cba76031519f";

        // First get the actual content directly through resolveDomainAndFetchUrl
        const directResponse = await urlFetcher.resolveDomainAndFetchUrl(
            {
                subdomain: siteObjectId,
                path: "/test.html",
            },
            siteObjectId,
        );
        expect(directResponse.status).toBe(200);
        const expectedContent = await directResponse.text();

        // Now test the routing flow
        const routedResponse = await urlFetcher.resolveDomainAndFetchUrl(
            {
                subdomain: siteObjectId,
                path: "/test",
            },
            siteObjectId,
        );
        expect(routedResponse.status).toBe(200);
        const actualContent = await routedResponse.text();

        // Verify we got the same content as direct fetch
        expect(actualContent).toBe(expectedContent);

        // Also fetch 404.html to prove we got different content
        const notFoundResponse = await urlFetcher.resolveDomainAndFetchUrl(
            {
                subdomain: siteObjectId,
                path: "/404.html",
            },
            siteObjectId,
        );
        expect(notFoundResponse.status).toBe(200);
        const notFoundContent = await notFoundResponse.text();

        // Verify we didn't get 404.html content
        expect(actualContent).not.toBe(notFoundContent);
    });

    test("should return portal fallback when site's 404.html blob is expired", async () => {
        const urlFetcher = new UrlFetcher(
            new ResourceFetcher(rpcSelector, sitePackage),
            new SuiNSResolver(rpcSelector),
            wsRouter,
            aggregatorUrl,
            true,
        );

        const fetchUrlSpy = vi.spyOn(urlFetcher, "fetchUrl");
        // The resource exists on-chain but the blob is expired on the aggregator.
        fetchUrlSpy.mockImplementation(
            async (objectId: string, path: string): Promise<FetchUrlResult> => {
                if (path === "/404.html") {
                    return {
                        kind: "error",
                        reason: FetchUrlFailReason.BlobUnavailable,
                        response: new Response("blob unavailable", { status: 404 }),
                    };
                }
                return {
                    kind: "notFound",
                    reason: FetchUrlFailReason.ResourceNotFound,
                };
            },
        );

        const getRoutesSpy = vi.spyOn(wsRouter, "getRoutes");
        getRoutesSpy.mockImplementation(async () => {
            return { routes_list: new Map() };
        });

        const siteObjectId = "0x0977d45a9adb8af8405c0698b0e049de05f8c89da75ca16ac6a6cba76031519f";

        const response = await urlFetcher.resolveDomainAndFetchUrl(
            { subdomain: siteObjectId, path: "/nonexistent" },
            siteObjectId,
        );

        // Should get the portal's custom404NotFound, NOT the blobUnavailable page
        expect(response.status).toBe(404);
        const text = await response.text();
        expect(text).not.toContain("no longer available");
        expect(text).toContain("Page not found");
    });
});
