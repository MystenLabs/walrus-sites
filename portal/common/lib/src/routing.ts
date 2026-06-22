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
import { matchGlob, validateGlobPattern, validateRegexPattern } from "@lib/route_patterns";

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
     * Matches the path to the appropriate route.
     * Uses the legacy regex pattern where `*` is converted to `.*` (matches
     * any characters including `/`). When multiple patterns match, the longest
     * pattern wins. Patterns that fail validation are skipped (and logged).
     *
     * Returns an object with:
     * - `match`: the matched route target, or undefined if no match.
     * - `regexOnlyPatterns`: patterns that matched via the legacy regex but NOT
     *   via the glob matcher. These will behave differently once routes migrate
     *   to glob matching; the caller logs them with site context as a canary.
     *
     * @param path - The path to match.
     * @param routes - The routes to match against.
     */
    public matchPathToRoute(
        path: string,
        routes: Routes,
    ): { match: string | undefined; regexOnlyPatterns: string[] } {
        logger.info(
            "Attempting to match the provided `path` with the routes in the Routes object",
            {
                path,
                routesDFList: routes.routes_list,
            },
        );
        if (routes.routes_list.size === 0) return { match: undefined, regexOnlyPatterns: [] };

        const regexOnlyPatterns: string[] = [];

        const filtered = Array.from(routes.routes_list.entries()).filter(([pattern]) => {
            const validation = validateRegexPattern(pattern);
            if (!validation.ok) {
                logger.warn("Skipping unsafe route pattern", {
                    path,
                    pattern,
                    reason: validation.reason,
                });
                return false;
            }
            const regexMatch = new RegExp(`^${pattern.replace(/\*/g, ".*")}$`).test(path);
            // Shadow-test glob matching to surface route patterns that won't
            // survive the migration from regex to glob matching.
            if (regexMatch && !matchGlob(pattern, path)) {
                regexOnlyPatterns.push(pattern);
            }
            return regexMatch;
        });

        if (filtered.length === 0) {
            logger.info("No matching routes found.", {
                path,
                routesDFList: routes.routes_list,
            });
            return { match: undefined, regexOnlyPatterns };
        }

        const match = filtered.reduce(
            (a, b) => (a[0].length >= b[0].length ? a : b),
            filtered[0],
        )[1];
        return { match, regexOnlyPatterns };
    }

    /**
     * Matches the path to a redirect entry using glob patterns.
     * When multiple patterns match, the longest pattern wins. Patterns that fail
     * validation are skipped (and logged).
     *
     * @param path - The path to match.
     * @param redirects - The redirects to match against.
     */
    public matchPathToRedirect(path: string, redirects: Redirects): Redirect | undefined {
        logger.info("Attempting to match the provided `path` with the redirects", { path });
        if (redirects.redirect_list.size === 0) return undefined;

        const filtered = Array.from(redirects.redirect_list.entries()).filter(([pattern]) => {
            const validation = validateGlobPattern(pattern);
            if (!validation.ok) {
                logger.warn("Skipping unsafe redirect pattern", {
                    path,
                    pattern,
                    reason: validation.reason,
                });
                return false;
            }
            return matchGlob(pattern, path);
        });

        if (filtered.length === 0) {
            logger.info("No matching redirects found.", { path });
            return undefined;
        }

        return filtered.reduce((a, b) => (a[0].length >= b[0].length ? a : b), filtered[0])[1];
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
