// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { Redirect, Redirects, Routes } from "@lib/types";
import { DynamicFieldStruct, RedirectsStruct, RoutesStruct } from "@lib/bcs_data_parsing";
import { bcs, BcsType } from "@mysten/bcs";
import logger from "@lib/logger";
import { RPCSelector } from "@lib/rpc_selector";
import { SuiClientTypes } from "@mysten/sui/client";
import { deriveDynamicFieldID } from "@mysten/sui/utils";
import { instrumentationFacade } from "@lib/instrumentation";
import {
    compareGlobSpecificity,
    matchGlob,
    regexToGlobPattern,
    validateGlobPattern,
    validateRegexPattern,
} from "@lib/path_patterns";

/**
 * The WalrusSitesRouter class is responsible for handling the routing and redirect
 * logic for published Walrus Sites, based on the `routes` and `redirects` fields
 * inside the `ws-resources.json`.
 */
export class WalrusSitesRouter {
    constructor(
        private rpcSelector: RPCSelector,
        private enableGlobRouting = false,
    ) {}

    /**
     * Gets both the Routes and Redirects dynamic fields in a single RPC call.
     *
     * @param siteObjectId - The ID of the site object.
     * @returns The routes and redirects; each is an `Error` when the dynamic
     * field couldn't be fetched (normally an expected miss — the site simply
     * doesn't define that optional field). Callers decide how to log it.
     */
    public async getRoutesAndRedirects(siteObjectId: string): Promise<{
        routes: Routes | Error;
        redirects: Redirects | Error;
    }> {
        const reqStartTime = Date.now();
        const routesDfId = this.deriveSiteFieldId(siteObjectId, "routes");
        const redirectsDfId = this.deriveSiteFieldId(siteObjectId, "redirects");
        logger.info("Fetching routes and redirects dynamic fields", {
            siteObjectId,
            routesDfId,
            redirectsDfId,
        });

        const responses = await this.rpcSelector.multiGetObjects([routesDfId, redirectsDfId], {
            content: true,
        });

        const rpcDuration = Date.now() - reqStartTime;
        instrumentationFacade.recordFetchRoutesAndRedirectsFieldObjectsTime(
            rpcDuration,
            siteObjectId,
        );

        const [routesRes, redirectsRes] = responses;
        const routes =
            routesRes instanceof Error
                ? routesRes
                : this.parseDynamicFieldValue(routesRes, RoutesStruct, "Routes");
        const redirects =
            redirectsRes instanceof Error
                ? redirectsRes
                : this.parseDynamicFieldValue(redirectsRes, RedirectsStruct, "Redirects");

        const totalDuration = Date.now() - reqStartTime;
        instrumentationFacade.recordRoutesAndRedirectsResolutionTime(totalDuration, siteObjectId);

        return { routes, redirects };
    }

    /**
     * Matches the path to the appropriate route. With the glob flag off,
     * patterns match via the legacy regex (`*` becomes `.*` and crosses `/`) and
     * the longest matching pattern wins; with it on, each pattern is rewritten to
     * its glob equivalent, matched via the glob matcher, and the most-specific
     * match wins. Either way, patterns that fail validation are skipped (and
     * logged). While the flag is off, the glob result is still computed and a
     * warning is logged when the two targets differ, so real divergences surface
     * in production logs before the flag flips.
     *
     * @param path - The path to match.
     * @param routes - The routes to match against.
     */
    public matchPathToRoute(path: string, routes: Routes): string | undefined {
        logger.info("Attempting to match the provided `path` with the routes", { path });
        if (routes.routes_list.size === 0) return undefined;

        let match: string | undefined;
        if (this.enableGlobRouting) {
            match = this.matchRouteViaGlob(path, routes);
        } else {
            match = this.matchRouteViaRegex(path, routes);
            // Migration canary — removed together with the flag.
            const globMatch = this.matchRouteViaGlob(path, routes);
            if (globMatch !== match) {
                logger.warn("Route target will change when glob routing is enabled", {
                    path,
                    regexTarget: match,
                    globTarget: globMatch,
                });
            }
        }

        if (match === undefined) {
            logger.info("No matching routes found.", { path });
        }
        return match;
    }

    /**
     * Legacy route matching: each `*` becomes `.*` and crosses `/`. Patterns are
     * validated first so a ReDoS-prone one is skipped (and logged) instead of
     * being handed to `RegExp`. The longest matching pattern wins.
     */
    private matchRouteViaRegex(path: string, routes: Routes): string | undefined {
        const matches = Array.from(routes.routes_list.entries()).filter(([pattern]) => {
            const validation = validateRegexPattern(pattern);
            if (!validation.ok) {
                logger.warn("Skipping unsafe route pattern", {
                    path,
                    pattern,
                    reason: validation.reason,
                });
                return false;
            }
            return new RegExp(`^${pattern.replaceAll("*", ".*")}$`).test(path);
        });
        if (matches.length === 0) return undefined;
        return matches.reduce((a, b) => (a[0].length >= b[0].length ? a : b), matches[0])[1];
    }

    /**
     * Glob route matching: each legacy pattern is rewritten to its glob
     * equivalent (so authored catch-all patterns keep their reach), then the most
     * specific matching glob wins.
     */
    private matchRouteViaGlob(path: string, routes: Routes): string | undefined {
        const globRoutes = Array.from(routes.routes_list, ([pattern, target]): [string, string] => [
            regexToGlobPattern(pattern),
            target,
        ]);
        return this.matchGlobEntry(globRoutes, path, "route");
    }

    /**
     * Matches the path to a redirect entry using glob patterns. Redirects are
     * authored as globs, so they are matched directly (no regex rewrite). The
     * most specific matching pattern wins; patterns that fail validation are
     * skipped (and logged).
     *
     * @param path - The path to match.
     * @param redirects - The redirects to match against.
     */
    public matchPathToRedirect(path: string, redirects: Redirects): Redirect | undefined {
        logger.info("Attempting to match the provided `path` with the redirects", { path });
        if (redirects.redirect_list.size === 0) return undefined;

        const match = this.matchGlobEntry(redirects.redirect_list, path, "redirect");
        if (match === undefined) {
            logger.info("No matching redirects found.", { path });
        }
        return match;
    }

    /**
     * Matches `path` against the glob `entries` ([glob, value] pairs) and returns
     * the value of the most specific match, or undefined if none match. Entries
     * whose glob fails validation are skipped (and logged). The most specific
     * glob wins — most literal characters, then fewest wildcards — with ties
     * resolved in iteration order. Routes rewrite their legacy patterns to globs
     * before calling; redirects are already globs.
     */
    private matchGlobEntry<V>(
        entries: Iterable<[string, V]>,
        path: string,
        kind: "route" | "redirect",
    ): V | undefined {
        let winner: { glob: string; value: V } | undefined;
        for (const [glob, value] of entries) {
            const validation = validateGlobPattern(glob);
            if (!validation.ok) {
                logger.warn(`Skipping unsafe ${kind} pattern`, {
                    path,
                    pattern: glob,
                    reason: validation.reason,
                });
                continue;
            }
            if (!matchGlob(glob, path)) continue;
            if (!winner || compareGlobSpecificity(glob, winner.glob) < 0) {
                winner = { glob, value };
            }
        }
        return winner?.value;
    }

    /**
     * Derives the dynamic field object ID for a site field.
     */
    private deriveSiteFieldId(siteObjectId: string, fieldName: string): string {
        return deriveDynamicFieldID(
            siteObjectId,
            "vector<u8>",
            bcs.vector(bcs.u8()).serialize(Buffer.from(fieldName)).toBytes(),
        );
    }

    /**
     * Parses a dynamic field value from a fetched object.
     * Throws if the object exists but has unexpected format.
     *
     * TODO(tech-debt): the throw rejects the whole getRoutesAndRedirects
     * promise, which fails the request. Now that the return channel already
     * carries `Routes | Error`, a malformed field could instead be returned as
     * an `Error` — the caller's warn branch would log it and the request would
     * degrade gracefully to the fallback chain. Kept as a throw for now to
     * avoid a behavior change in this PR.
     */
    private parseDynamicFieldValue<T>(
        response: SuiClientTypes.Object<{ content: true }>,
        valueStruct: BcsType<T>,
        fieldName: string,
    ): T {
        if (response.content) {
            const df = DynamicFieldStruct(bcs.vector(bcs.u8()), valueStruct).parse(
                response.content,
            );
            return df.value as any as T;
        }
        logger.warn(`${fieldName} dynamic field has unexpected format`, { response });
        throw new Error(`${fieldName} object data could not be fetched.`);
    }
}
