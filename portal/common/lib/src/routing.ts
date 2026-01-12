// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { SuiObjectResponse } from "@mysten/sui/client";
import { Routes } from "@lib/types";
import { DynamicFieldStruct, RoutesStruct } from "@lib/bcs_data_parsing";
import { bcs, fromBase64 } from "@mysten/bcs";
import logger from "@lib/logger";
import { RPCSelector } from "@lib/rpc_selector";
import { deriveDynamicFieldID } from "@mysten/sui/utils";
import { instrumentationFacade } from "@lib/instrumentation";

/**
 * The WalrusSiteRouter class is responsible for handling the routing logic for published
 * Walrus Sites, depending by the definition of the `routes` field inside the `ws-resources.json`.
 */
export class WalrusSitesRouter {
    constructor(private rpcSelector: RPCSelector) {}

    /**
     * Gets the Routes dynamic field of the site object.
     * Returns the extracted routes_list map to use for future requests,
     * and redirects the paths matched accordingly.
     *
     * @param siteObjectId - The ID of the site object.
     * @returns The routes list.
     */
    public async getRoutes(siteObjectId: string): Promise<Routes | undefined> {
        const reqStartTime = Date.now();

        logger.info("Retrieving the Routes dynamic field object (if present) associated with the Site object", { siteObjectId });
        const routesObj = await this.fetchRoutesDynamicFieldObject(siteObjectId);
        const objectData = routesObj.data;
        if (objectData && objectData.bcs && objectData.bcs.dataType === "moveObject") {
            const routingDuration = Date.now() - reqStartTime;
            instrumentationFacade.recordRoutingTime(routingDuration, siteObjectId);
            return this.parseRoutesData(objectData.bcs.bcsBytes);
        }
        if (!objectData) {
            logger.warn(
                "Routes dynamic field does not contain a `data` field.",
            );
            return;
        } else if (!objectData.bcs) {
            logger.warn(
                "Routes dynamic field does not contain a `bcs` field.",
            );
        } else if (!objectData.bcs.dataType) {
            logger.warn(
                "Routes dynamic field does not contain a `dataType` field."
            );
        }
        throw new Error("Routes object data could not be fetched.");
    }

    /**
     * Derives and fetches the Routes dynamic field object.
     *
     * @param siteObjectId - The site object ID.
     * @returns The routes object.
     */
    private async fetchRoutesDynamicFieldObject(siteObjectId: string): Promise<SuiObjectResponse> {
        const reqStartTime = Date.now();
        const routesMoveType = "vector<u8>";
        const dynamicFieldId = deriveDynamicFieldID(
            siteObjectId,
            routesMoveType,
            bcs.vector(bcs.u8()).serialize(Buffer.from("routes")).toBytes(),
        );
        const dfObjectResponse = await this.rpcSelector.getObject({
            id: dynamicFieldId,
            options: { showBcs: true },
        });
        const fetchRoutesDynamicFieldObjectDuration = Date.now() - reqStartTime;
		instrumentationFacade.recordFetchRoutesDynamicFieldObjectTime(
			fetchRoutesDynamicFieldObjectDuration,
			siteObjectId,
		);
        return dfObjectResponse;
    }

    /**
     * Parses the routes data from the BCS bytes.
     *
     * @param bcsBytes - The BCS bytes of the routes object.
     * @returns The parsed routes data.
     */
    private parseRoutesData(bcsBytes: string): Routes {
        const df = DynamicFieldStruct(
            // BCS declaration of the ROUTES_FIELD in site.move.
            bcs.vector(bcs.u8()),
            // The value of the df, i.e. the Routes Struct.
            RoutesStruct,
        ).parse(fromBase64(bcsBytes));

        return df.value as any as Routes;
    }

    /**
     * Matches the path to the appropriate route.
     * Path patterns in the routes list are sorted by length in descending order.
     * Then the first match is returned.
     *
     * @param path - The path to match.
     * @param routes - The routes to match against.
     */
    public matchPathToRoute(path: string, routes: Routes): string | undefined {
    	logger.info("Attempting to match the provided `path` with the routes in the Routes object", {path, routesDFList: routes.routes_list})
        if (routes.routes_list.size == 0) {
            // If the map is empty there is no match.
            return undefined;
        }

        const filteredRoutes = Array.from(routes.routes_list.entries())
                .filter(([pattern, _]) => new RegExp(`^${pattern.replace(/\*/g, ".*")}$`).test(path));

		if (filteredRoutes.length === 0) {
			logger.warn("No matching routes found.", { path, routesDFList: routes.routes_list });
            return undefined;
        }

        const res = filteredRoutes.reduce((a, b) => (a[0].length >= b[0].length ? a : b));

        return res[1];
    }
}
