// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { WalrusSitesRouter } from "@lib/routing";
import { test, expect, describe, vi, beforeEach } from "vitest";
import { RPCSelector } from "@lib/rpc_selector";
import { UrlFetcher } from "@lib/url_fetcher";
import type { FetchUrlResult } from "@lib/url_fetcher";
import { ResourceFetcher } from "@lib/resource";
import { SuiNSResolver } from "@lib/suins";
import { parsePriorityUrlList, PriorityExecutor } from "@lib/priority_executor";
import { Redirect, Redirects } from "@lib/types";

const rpcPriorityUrls = parsePriorityUrlList(process.env.RPC_URL_LIST!);
const rpcSelector = new RPCSelector(rpcPriorityUrls, "testnet");
const wsRouter = new WalrusSitesRouter(rpcSelector);
const aggregatorPriorityUrls = parsePriorityUrlList(process.env.AGGREGATOR_URL_LIST!);
const sitePackage = process.env.SITE_PACKAGE;

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
        const { match } = wsRouter.matchPathToRoute(requestPath, routesExample);
        expect(match).toEqual(expected);
    });
});

// Test in the case there are no routes.
const emptyRoutes = { routes_list: new Map<string, string>() };

testCases.forEach(([requestPath, _]) => {
    test(`matchPathToRoute: empty routes for "${requestPath}"`, () => {
        const { match } = wsRouter.matchPathToRoute(requestPath, emptyRoutes);
        expect(match).toEqual(undefined);
    });
});

// --- Redirect matching tests ---

const redirectsExample: Redirects = {
    redirect_list: new Map<string, Redirect>([
        ["/old-page", { location: "/new-page", status_code: 301 }],
        ["/temp", { location: "https://example.com/temp", status_code: 302 }],
        ["/blog/old-*", { location: "/blog/archive", status_code: 308 }],
        ["/docs/*", { location: "/documentation", status_code: 307 }],
    ]),
};

describe("matchPathToRedirect", () => {
    test("exact match returns redirect", () => {
        const match = wsRouter.matchPathToRedirect("/old-page", redirectsExample);
        expect(match).toEqual({ location: "/new-page", status_code: 301 });
    });

    test("glob match returns redirect", () => {
        const match = wsRouter.matchPathToRedirect("/blog/old-post", redirectsExample);
        expect(match).toEqual({ location: "/blog/archive", status_code: 308 });
    });

    test("longest glob match wins", () => {
        // /blog/old-post matches /blog/old-* (length 11) but not /docs/* (doesn't match)
        const match = wsRouter.matchPathToRedirect("/blog/old-post", redirectsExample);
        expect(match).toEqual({ location: "/blog/archive", status_code: 308 });
    });

    test("no match returns undefined", () => {
        const match = wsRouter.matchPathToRedirect("/nonexistent", redirectsExample);
        expect(match).toBeUndefined();
    });

    test("empty redirects returns undefined", () => {
        const emptyRedirects: Redirects = { redirect_list: new Map() };
        const match = wsRouter.matchPathToRedirect("/old-page", emptyRedirects);
        expect(match).toBeUndefined();
    });

    test("preserves different status codes", () => {
        expect(wsRouter.matchPathToRedirect("/old-page", redirectsExample)?.status_code).toBe(301);
        expect(wsRouter.matchPathToRedirect("/temp", redirectsExample)?.status_code).toBe(302);
        expect(wsRouter.matchPathToRedirect("/docs/guide", redirectsExample)?.status_code).toBe(
            307,
        );
        expect(wsRouter.matchPathToRedirect("/blog/old-x", redirectsExample)?.status_code).toBe(
            308,
        );
    });
});

// --- Integration tests ---

const siteObjectId = "0x0977d45a9adb8af8405c0698b0e049de05f8c89da75ca16ac6a6cba76031519f";

function makeUrlFetcher(): UrlFetcher {
    return new UrlFetcher(
        new ResourceFetcher(rpcSelector, sitePackage!),
        new SuiNSResolver(rpcSelector),
        wsRouter,
        new PriorityExecutor(aggregatorPriorityUrls),
        true,
    );
}

describe("routing integration tests", () => {
    beforeEach(() => {
        vi.clearAllMocks();
    });

    test("should check routes before 404.html", async () => {
        const urlFetcher = makeUrlFetcher();

        const fetchUrlSpy = vi.spyOn(urlFetcher, "fetchUrl");
        fetchUrlSpy.mockImplementation(
            async (_objectId: string, path: string): Promise<FetchUrlResult> => {
                switch (path) {
                    case "/test.html":
                        return {
                            status: "Ok",
                            response: new Response("test.html content", { status: 200 }),
                        };
                    case "/404.html":
                        return {
                            status: "Ok",
                            response: new Response("404 page content", { status: 200 }),
                        };
                    default:
                        return { status: "ResourceNotFound" };
                }
            },
        );

        const getRoutesAndRedirectsSpy = vi.spyOn(wsRouter, "getRoutesAndRedirects");
        getRoutesAndRedirectsSpy.mockImplementation(async () => ({
            routes: { routes_list: new Map([["/test", "/test.html"]]) },
            redirects: undefined,
        }));

        // First get the actual content directly
        const directResponse = await urlFetcher.resolveDomainAndFetchUrl(
            { subdomain: siteObjectId, path: "/test.html" },
            siteObjectId,
        );
        expect(directResponse.status).toBe(200);
        const expectedContent = await directResponse.text();

        // Now test the routing flow
        const routedResponse = await urlFetcher.resolveDomainAndFetchUrl(
            { subdomain: siteObjectId, path: "/test" },
            siteObjectId,
        );
        expect(routedResponse.status).toBe(200);
        const actualContent = await routedResponse.text();
        expect(actualContent).toBe(expectedContent);

        // Also fetch 404.html to prove we got different content
        const notFoundResponse = await urlFetcher.resolveDomainAndFetchUrl(
            { subdomain: siteObjectId, path: "/404.html" },
            siteObjectId,
        );
        expect(notFoundResponse.status).toBe(200);
        const notFoundContent = await notFoundResponse.text();
        expect(actualContent).not.toBe(notFoundContent);
    });

    test("should return portal fallback when site's 404.html blob is expired", async () => {
        const urlFetcher = new UrlFetcher(
            new ResourceFetcher(rpcSelector, sitePackage!),
            new SuiNSResolver(rpcSelector),
            wsRouter,
            new PriorityExecutor([{ url: "http://unused", retries: 0, metric: 100 }]),
            true,
        );

        const fetchUrlSpy = vi.spyOn(urlFetcher, "fetchUrl");
        fetchUrlSpy.mockImplementation(
            async (_objectId: string, path: string): Promise<FetchUrlResult> => {
                if (path === "/404.html") {
                    return {
                        status: "BlobUnavailable",
                        response: new Response("blob unavailable", { status: 404 }),
                    };
                }
                return { status: "ResourceNotFound" };
            },
        );

        const getRoutesAndRedirectsSpy = vi.spyOn(wsRouter, "getRoutesAndRedirects");
        getRoutesAndRedirectsSpy.mockImplementation(async () => ({
            routes: { routes_list: new Map() },
            redirects: undefined,
        }));

        const response = await urlFetcher.resolveDomainAndFetchUrl(
            { subdomain: siteObjectId, path: "/nonexistent" },
            siteObjectId,
        );

        expect(response.status).toBe(404);
        const text = await response.text();
        expect(text).not.toContain("no longer available");
        expect(text).toContain("Page not found");
    });

    test("should return redirect response when redirect matches", async () => {
        const urlFetcher = makeUrlFetcher();

        const fetchUrlSpy = vi.spyOn(urlFetcher, "fetchUrl");
        fetchUrlSpy.mockImplementation(async (): Promise<FetchUrlResult> => {
            return { status: "ResourceNotFound" };
        });

        const getRoutesAndRedirectsSpy = vi.spyOn(wsRouter, "getRoutesAndRedirects");
        getRoutesAndRedirectsSpy.mockImplementation(async () => ({
            routes: { routes_list: new Map([["/*", "/index.html"]]) },
            redirects: {
                redirect_list: new Map<string, Redirect>([
                    ["/old", { location: "/new", status_code: 301 }],
                ]),
            },
        }));

        const response = await urlFetcher.resolveDomainAndFetchUrl(
            { subdomain: siteObjectId, path: "/old" },
            siteObjectId,
        );

        // Should return a 301 redirect, NOT try to fetch /index.html via route matching
        expect(response.status).toBe(301);
        expect(response.headers.get("Location")).toBe("/new");
    });

    test("should fall through to routes when no redirect matches", async () => {
        const urlFetcher = makeUrlFetcher();

        const fetchUrlSpy = vi.spyOn(urlFetcher, "fetchUrl");
        fetchUrlSpy.mockImplementation(
            async (_objectId: string, path: string): Promise<FetchUrlResult> => {
                if (path === "/index.html") {
                    return {
                        status: "Ok",
                        response: new Response("index content", { status: 200 }),
                    };
                }
                return { status: "ResourceNotFound" };
            },
        );

        const getRoutesAndRedirectsSpy = vi.spyOn(wsRouter, "getRoutesAndRedirects");
        getRoutesAndRedirectsSpy.mockImplementation(async () => ({
            routes: { routes_list: new Map([["/*", "/index.html"]]) },
            redirects: {
                redirect_list: new Map<string, Redirect>([
                    ["/old", { location: "/new", status_code: 301 }],
                ]),
            },
        }));

        // Request a path that doesn't match any redirect but matches the route
        const response = await urlFetcher.resolveDomainAndFetchUrl(
            { subdomain: siteObjectId, path: "/some-page" },
            siteObjectId,
        );

        expect(response.status).toBe(200);
        const text = await response.text();
        expect(text).toBe("index content");
    });

    test("should handle redirect with external URL", async () => {
        const urlFetcher = makeUrlFetcher();

        const fetchUrlSpy = vi.spyOn(urlFetcher, "fetchUrl");
        fetchUrlSpy.mockImplementation(async (): Promise<FetchUrlResult> => {
            return { status: "ResourceNotFound" };
        });

        const getRoutesAndRedirectsSpy = vi.spyOn(wsRouter, "getRoutesAndRedirects");
        getRoutesAndRedirectsSpy.mockImplementation(async () => ({
            routes: undefined,
            redirects: {
                redirect_list: new Map<string, Redirect>([
                    ["/external", { location: "https://example.com/page", status_code: 302 }],
                ]),
            },
        }));

        const response = await urlFetcher.resolveDomainAndFetchUrl(
            { subdomain: siteObjectId, path: "/external" },
            siteObjectId,
        );

        expect(response.status).toBe(302);
        expect(response.headers.get("Location")).toBe("https://example.com/page");
    });
});
