// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { SuiObjectResponse } from "@mysten/sui/jsonRpc";
import { Redirect, Redirects, Routes } from "@lib/types";
import { DynamicFieldStruct, RedirectsStruct, RoutesStruct } from "@lib/bcs_data_parsing";
import { bcs, BcsType, fromBase64 } from "@mysten/bcs";
import logger from "@lib/logger";
import { RPCSelector } from "@lib/rpc_selector";
import { deriveDynamicFieldID } from "@mysten/sui/utils";
import { instrumentationFacade } from "@lib/instrumentation";
import picomatch from "picomatch";

/**
 * The WalrusSitesRouter class is responsible for handling the routing and redirect
 * logic for published Walrus Sites, based on the `routes` and `redirects` fields
 * inside the `ws-resources.json`.
 */
export class WalrusSitesRouter {
    constructor(private rpcSelector: RPCSelector) {}

    /**
     * Gets the Routes dynamic field of the site object.
     *
     * @param siteObjectId - The ID of the site object.
     * @returns The routes list, or undefined if not present.
     */
    public async getRoutes(siteObjectId: string): Promise<Routes | undefined> {
        const reqStartTime = Date.now();
        const dfId = this.deriveSiteFieldId(siteObjectId, "routes");
        const response = await this.rpcSelector.getObject({
            id: dfId,
            options: { showBcs: true },
        });
        const routingDuration = Date.now() - reqStartTime;
        instrumentationFacade.recordRoutingTime(routingDuration, siteObjectId);
        return this.parseDynamicFieldValue(response, RoutesStruct, "Routes");
    }

    /**
     * Gets both the Routes and Redirects dynamic fields in a single RPC call.
     *
     * @param siteObjectId - The ID of the site object.
     * @returns The routes and redirects, each undefined if not present.
     */
    public async getRoutesAndRedirects(siteObjectId: string): Promise<{
        routes: Routes | undefined;
        redirects: Redirects | undefined;
    }> {
        const reqStartTime = Date.now();
        const routesDfId = this.deriveSiteFieldId(siteObjectId, "routes");
        const redirectsDfId = this.deriveSiteFieldId(siteObjectId, "redirects");

        const responses = await this.rpcSelector.multiGetObjects({
            ids: [routesDfId, redirectsDfId],
            options: { showBcs: true },
        });

        const routingDuration = Date.now() - reqStartTime;
        instrumentationFacade.recordRoutingTime(routingDuration, siteObjectId);

        const routes = this.parseDynamicFieldValue(responses[0], RoutesStruct, "Routes");
        const redirects = this.parseDynamicFieldValue(responses[1], RedirectsStruct, "Redirects");

        if (redirects) {
            this.warnOnRedirectLoops(redirects);
        }

        return { routes, redirects };
    }

    /**
     * Matches the path to the appropriate route.
     * Uses the legacy regex pattern where `*` is converted to `.*` (matches
     * any characters including `/`). When multiple patterns match, the longest
     * pattern wins.
     *
     * @param path - The path to match.
     * @param routes - The routes to match against.
     */
    public matchPathToRoute(path: string, routes: Routes): string | undefined {
        logger.info(
            "Attempting to match the provided `path` with the routes in the Routes object",
            {
                path,
                routesDFList: routes.routes_list,
            },
        );
        if (routes.routes_list.size === 0) return undefined;

        const filtered = Array.from(routes.routes_list.entries()).filter(([pattern]) => {
            const regexMatch = new RegExp(`^${pattern.replace(/\*/g, ".*")}$`).test(path);
            if (regexMatch && !picomatch(pattern, { dot: true })(path)) {
                logger.warn("Route pattern matches via regex but not via glob (picomatch)", {
                    pattern,
                    path,
                });
            }
            return regexMatch;
        });

        if (filtered.length === 0) {
            logger.warn("No matching routes found.", {
                path,
                routesDFList: routes.routes_list,
            });
            return undefined;
        }

        return filtered.reduce((a, b) => (a[0].length >= b[0].length ? a : b))[1];
    }

    /**
     * Matches the path to a redirect entry using glob patterns (picomatch).
     * When multiple patterns match, the longest pattern wins.
     *
     * @param path - The path to match.
     * @param redirects - The redirects to match against.
     */
    public matchPathToRedirect(path: string, redirects: Redirects): Redirect | undefined {
        logger.info("Attempting to match the provided `path` with the redirects", { path });
        if (redirects.redirect_list.size === 0) return undefined;

        const filtered = Array.from(redirects.redirect_list.entries()).filter(([pattern]) =>
            picomatch(pattern, { dot: true })(path),
        );

        if (filtered.length === 0) {
            logger.warn("No matching redirects found.", { path });
            return undefined;
        }

        return filtered.reduce((a, b) => (a[0].length >= b[0].length ? a : b))[1];
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
     * Parses a dynamic field value from a SuiObjectResponse.
     * Returns undefined if the DF doesn't exist on-chain.
     * Throws if the object exists but has unexpected format.
     */
    private parseDynamicFieldValue<T>(
        response: SuiObjectResponse,
        valueStruct: BcsType<T>,
        fieldName: string,
    ): T | undefined {
        const objectData = response.data;
        if (objectData?.bcs?.dataType === "moveObject") {
            const df = DynamicFieldStruct(bcs.vector(bcs.u8()), valueStruct).parse(
                fromBase64(objectData.bcs.bcsBytes),
            );
            return df.value as any as T;
        }
        if (!objectData) {
            return undefined;
        }
        logger.warn(`${fieldName} dynamic field has unexpected format`, { objectData });
        throw new Error(`${fieldName} object data could not be fetched.`);
    }

    /**
     * Logs a warning if any redirect's location matches another redirect pattern,
     * indicating a possible redirect loop.
     */
    private warnOnRedirectLoops(redirects: Redirects): void {
        for (const [path, redirect] of redirects.redirect_list) {
            const match = Array.from(redirects.redirect_list.entries()).find(([pattern]) =>
                picomatch(pattern, { dot: true })(redirect.location),
            );
            if (match) {
                logger.warn("Possible redirect loop detected", {
                    from: path,
                    to: redirect.location,
                });
            }
        }
    }
}
