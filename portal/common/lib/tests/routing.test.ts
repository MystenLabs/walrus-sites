// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { WalrusSitesRouter } from "@lib/routing";
import { test, expect, describe, vi, beforeEach, afterEach } from "vitest";
import { isObjectNotFoundError, RPCSelector } from "@lib/rpc_selector";
import { SuiClientTypes } from "@mysten/sui/client";
import { UrlFetcher } from "@lib/url_fetcher";
import type { FetchUrlResult } from "@lib/url_fetcher";
import { ResourceFetcher } from "@lib/resource";
import { SuiNSResolver } from "@lib/suins";
import { parsePriorityUrlList, PriorityExecutor } from "@lib/priority_executor";
import { Redirect, Redirects } from "@lib/types";
import { DynamicFieldStruct, RoutesStruct, RedirectsStruct } from "@lib/bcs_data_parsing";
import { bcs, type BcsType } from "@mysten/bcs";
import logger from "@lib/logger";

const snakeSiteObjectId = "0x7a95e4be3948415b852fb287d455166a276d7a52f1a567b4a26b6b5e9c753158";
const rpcPriorityUrls = parsePriorityUrlList(process.env.RPC_URL_LIST!);
const rpcSelector = new RPCSelector(rpcPriorityUrls, "testnet");
const wsRouter = new WalrusSitesRouter(rpcSelector);
const aggregatorPriorityUrls = parsePriorityUrlList(process.env.AGGREGATOR_URL_LIST!);
const sitePackage = process.env.ORIGINAL_PACKAGE_ID;

/**
 * Encodes a value as a BCS DynamicField and returns the raw bytes, matching the
 * `content` returned by `multiGetObjects(ids, { content: true })`.
 */
function encodeDynamicField<T>(fieldName: string, valueStruct: BcsType<T>, value: T): Uint8Array {
    const parentId = "0000000000000000000000000000000000000000000000000000000000000000";
    const df = DynamicFieldStruct(bcs.vector(bcs.u8()), valueStruct);
    return df
        .serialize({
            parentId,
            name: Array.from(Buffer.from(fieldName)),
            value,
        })
        .toBytes();
}

// TODO(tech-debt): partial mock — cast because SuiClientTypes.Object also requires
// owner/type/previousTransaction/objectBcs/json, unused by these tests.
function makeBcsObjectResponse(content: Uint8Array): SuiClientTypes.Object<{ content: true }> {
    return {
        objectId: "0x1",
        version: "1",
        digest: "test",
        content,
    } as SuiClientTypes.Object<{ content: true }>;
}

describe("getRoutesAndRedirects", () => {
    afterEach(() => {
        vi.restoreAllMocks();
    });

    test("returns the Error elements when dynamic fields don't exist", async () => {
        const spy = vi.spyOn(rpcSelector, "multiGetObjects");
        // A missing dynamic field comes back as an `Error` element (gRPC core API).
        const routesMiss = new Error("Object 0xab not found");
        const redirectsMiss = new Error("Object 0xcd not found");
        spy.mockResolvedValue([routesMiss, redirectsMiss]);

        const result = await wsRouter.getRoutesAndRedirects(snakeSiteObjectId);
        // The Error slots are passed through untouched, and their shape is the
        // one callers recognize as an expected miss (info log, not warn).
        expect(result.routes).toBe(routesMiss);
        expect(result.redirects).toBe(redirectsMiss);
        expect(isObjectNotFoundError(result.routes)).toBe(true);
        expect(isObjectNotFoundError(result.redirects)).toBe(true);
        expect(spy).toHaveBeenCalledOnce();
        expect(spy).toHaveBeenCalledWith([expect.any(String), expect.any(String)], {
            content: true,
        });
    });

    test("parses routes and redirects from BCS data", async () => {
        const routesBcs = encodeDynamicField("routes", RoutesStruct, {
            routes_list: new Map([["/*", "/index.html"]]),
        });
        const redirectsBcs = encodeDynamicField("redirects", RedirectsStruct, {
            redirect_list: new Map([["/old", { location: "/new", status_code: 301 }]]),
        });

        const spy = vi.spyOn(rpcSelector, "multiGetObjects");
        spy.mockResolvedValue([
            makeBcsObjectResponse(routesBcs),
            makeBcsObjectResponse(redirectsBcs),
        ]);

        const result = await wsRouter.getRoutesAndRedirects(snakeSiteObjectId);

        const { routes, redirects } = result;
        if (routes instanceof Error || redirects instanceof Error) {
            throw new Error("expected parsed routes and redirects, got an Error element");
        }
        expect(routes.routes_list.get("/*")).toBe("/index.html");
        expect(redirects.redirect_list.get("/old")).toEqual({
            location: "/new",
            status_code: 301,
        });
    });

    test("throws on unexpected object format", async () => {
        const spy = vi.spyOn(rpcSelector, "multiGetObjects");
        spy.mockResolvedValue([
            // present object but no `content` — unexpected format
            { objectId: "0x1", version: "1", digest: "test" } as SuiClientTypes.Object<{
                content: true;
            }>,
            new Error("Object 0xredirects not found"),
        ]);

        await expect(wsRouter.getRoutesAndRedirects(snakeSiteObjectId)).rejects.toThrow(
            "Routes object data could not be fetched.",
        );
    });
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

describe("matchPathToRoute glob-divergence canary (flag off)", () => {
    afterEach(() => {
        vi.restoreAllMocks();
    });

    test("warns when the glob matcher would pick a different target", () => {
        const warnSpy = vi.spyOn(logger, "warn").mockImplementation(() => {});
        // Glued trailing star: the regex `^/api.*$` crosses `/`; the glob
        // rewrite leaves `/api*` single-segment, so the target changes.
        const routes = { routes_list: new Map<string, string>([["/api*", "/api.html"]]) };
        expect(wsRouter.matchPathToRoute("/api/x/y", routes)).toBe("/api.html");
        expect(warnSpy).toHaveBeenCalledWith(
            "Route target will change when glob routing is enabled",
            { path: "/api/x/y", regexTarget: "/api.html", globTarget: undefined },
        );
    });

    test("stays silent when both matchers agree", () => {
        const warnSpy = vi.spyOn(logger, "warn").mockImplementation(() => {});
        expect(wsRouter.matchPathToRoute("/somewhere/else", routesExample)).toBe("/else.jpeg");
        expect(warnSpy).not.toHaveBeenCalled();
    });
});

describe("matchPathToRoute skips unsafe patterns", () => {
    test("an over-cap route pattern is skipped, not matched", () => {
        const routes = {
            routes_list: new Map<string, string>([
                ["/a*b*c*", "/bad.html"], // 3 stars total -> skipped before the regex runs
                ["/*", "/ok.html"],
            ]),
        };
        // The unsafe pattern is longer, so if it were matched it would win.
        // Getting the catch-all proves it was skipped.
        const match = wsRouter.matchPathToRoute("/aXbYcZ", routes);
        expect(match).toBe("/ok.html");
    });
});

// --- Glob route matching (ENABLE_GLOB_ROUTING) ---

const globRouter = new WalrusSitesRouter(rpcSelector, true);

// A catch-all widens to require the extra slash, so every legacy regex case
// above resolves identically under the glob matcher (no behaviour drift).
testCases.forEach(([requestPath, expected]) => {
    test(`matchPathToRoute (glob): "${requestPath}" -> "${expected}"`, () => {
        expect(globRouter.matchPathToRoute(requestPath, routesExample)).toEqual(expected);
    });
});

describe("matchPathToRoute with glob routing enabled", () => {
    test("a catch-all `/*` matches any path below root", () => {
        const routes = { routes_list: new Map<string, string>([["/*", "/index.html"]]) };
        expect(globRouter.matchPathToRoute("/a/b/c/d", routes)).toBe("/index.html");
        expect(globRouter.matchPathToRoute("/x", routes)).toBe("/index.html");
    });

    test("a mid-pattern `*` stays within a single segment", () => {
        const routes = {
            routes_list: new Map<string, string>([["/forms/*/admin", "/admin.html"]]),
        };
        expect(globRouter.matchPathToRoute("/forms/contact/admin", routes)).toBe("/admin.html");
        // The legacy regex would cross `/` here; glob does not.
        expect(globRouter.matchPathToRoute("/forms/a/b/admin", routes)).toBeUndefined();
    });

    test("a within-segment `*` matches a prefix in one segment only", () => {
        const routes = { routes_list: new Map<string, string>([["/blog/old-*", "/archive.html"]]) };
        expect(globRouter.matchPathToRoute("/blog/old-post", routes)).toBe("/archive.html");
        expect(globRouter.matchPathToRoute("/blog/old/post", routes)).toBeUndefined();
    });

    test("a more specific prefix route beats the catch-all", () => {
        const routes = {
            routes_list: new Map<string, string>([
                ["/*", "/catch-all.html"],
                ["/docs/*", "/docs.html"],
            ]),
        };
        expect(globRouter.matchPathToRoute("/docs/intro", routes)).toBe("/docs.html");
    });

    test("an exact route is not shadowed by a sibling `/*` route", () => {
        // `/section/*` widens to require a deeper segment, so the bare `/section`
        // resolves to its own exact route — not the sibling catch-all.
        const routes = {
            routes_list: new Map<string, string>([
                ["/section", "/exact.html"],
                ["/section/*", "/child.html"],
            ]),
        };
        expect(globRouter.matchPathToRoute("/section", routes)).toBe("/exact.html");
        expect(globRouter.matchPathToRoute("/section/sub", routes)).toBe("/child.html");
    });

    test("an exact `/foo/` beats its sibling `/foo/*`, and neither matches `/foo`", () => {
        const routes = {
            routes_list: new Map<string, string>([
                ["/foo/*", "/catch.html"],
                ["/foo/", "/exact.html"],
            ]),
        };
        // `/foo/` is the most specific match for the trailing-slash path.
        expect(globRouter.matchPathToRoute("/foo/", routes)).toBe("/exact.html");
        // Only the catch-all matches a deeper path.
        expect(globRouter.matchPathToRoute("/foo/bar", routes)).toBe("/catch.html");
        // The widened `/foo/*` (-> `/foo/**/*`) does not match the bare prefix.
        expect(globRouter.matchPathToRoute("/foo", routes)).toBeUndefined();
    });

    test("a globstar plus a segment beats a bare globstar for a deep path", () => {
        // `/something/else/**/*` has one more literal `/` than `/something/else/**`,
        // so the deeper, more explicit pattern wins where both match.
        const routes = {
            routes_list: new Map<string, string>([
                ["/something/else/**", "/glob.html"],
                ["/something/else/**/*", "/glob-deep.html"],
            ]),
        };
        expect(globRouter.matchPathToRoute("/something/else/foo/bar", routes)).toBe(
            "/glob-deep.html",
        );
    });

    test("equally specific patterns resolve deterministically to the first", () => {
        // `/a/*/c` and `/*/b/c` both match `/a/b/c` with the same literal and
        // star counts, so the first-defined one wins.
        const routes = {
            routes_list: new Map<string, string>([
                ["/a/*/c", "/first.html"],
                ["/*/b/c", "/second.html"],
            ]),
        };
        expect(globRouter.matchPathToRoute("/a/b/c", routes)).toBe("/first.html");
    });

    test("an unsafe pattern is skipped under glob too", () => {
        const routes = {
            routes_list: new Map<string, string>([
                ["/x/a*a*a*/y", "/bad.html"], // more than one star in a segment -> skipped
                ["/*", "/ok.html"],
            ]),
        };
        expect(globRouter.matchPathToRoute("/x/aaa/y", routes)).toBe("/ok.html");
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

    test("most specific redirect wins over a broader sibling", () => {
        // `/blog/` and `/blog/*` both match `/blog/`; the exact one is more
        // specific (same literals, no wildcard), so it wins.
        const redirects: Redirects = {
            redirect_list: new Map<string, Redirect>([
                ["/blog/", { location: "/blog-index", status_code: 301 }],
                ["/blog/*", { location: "/blog-catch", status_code: 302 }],
            ]),
        };
        expect(wsRouter.matchPathToRedirect("/blog/", redirects)).toEqual({
            location: "/blog-index",
            status_code: 301,
        });
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

describe("matchPathToRedirect skips unsafe patterns", () => {
    test("an over-cap redirect pattern is skipped", () => {
        const redirects: Redirects = {
            redirect_list: new Map<string, Redirect>([
                ["/a*b*c*", { location: "/bad", status_code: 301 }], // 3 stars in one segment
            ]),
        };
        expect(wsRouter.matchPathToRedirect("/aXbYcZ", redirects)).toBeUndefined();
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
            redirects: new Error("Object 0xcd not found"),
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
            redirects: new Error("Object 0xcd not found"),
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
            routes: new Error("Object 0xab not found"),
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

    test("should return 508 on redirect self-loop", async () => {
        const urlFetcher = makeUrlFetcher();

        const fetchUrlSpy = vi.spyOn(urlFetcher, "fetchUrl");
        fetchUrlSpy.mockImplementation(async (): Promise<FetchUrlResult> => {
            return { status: "ResourceNotFound" };
        });

        const getRoutesAndRedirectsSpy = vi.spyOn(wsRouter, "getRoutesAndRedirects");
        getRoutesAndRedirectsSpy.mockImplementation(async () => ({
            routes: new Error("Object 0xab not found"),
            redirects: {
                redirect_list: new Map<string, Redirect>([
                    ["/loop", { location: "/loop", status_code: 301 }],
                ]),
            },
        }));

        const response = await urlFetcher.resolveDomainAndFetchUrl(
            { subdomain: siteObjectId, path: "/loop" },
            siteObjectId,
        );

        expect(response.status).toBe(508);
        const text = await response.text();
        expect(text).toContain("Redirect loop detected");
    });
});
