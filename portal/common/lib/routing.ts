// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { SuiObjectResponse } from "@mysten/sui/client";
import { Routes } from "./types";
import { DynamicFieldStruct, RoutesStruct } from "./bcs_data_parsing";
import { bcs, fromBase64 } from "@mysten/bcs";
import logger from "./logger";
import { RPCSelector } from "./rpc_selector";

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
    public async getRoutes(
        siteObjectId: string,
    ): Promise<Routes | undefined> {
        logger.info({ message: "Fetching routes dynamic field.", siteObjectId })
        const routesDF = await this.fetchRoutesDynamicField(siteObjectId);
        if (!routesDF.data) {
            logger.warn({
                message: "No routes dynamic field found for site object. Exiting getRoutes.",
                siteObjectId
            });
            return;
        }
        const routesObj = await this.fetchRoutesObject(routesDF.data.objectId);
        const objectData = routesObj.data;
        if (objectData && objectData.bcs && objectData.bcs.dataType === "moveObject") {
            return this.parseRoutesData(objectData.bcs.bcsBytes);
        }
        if (!objectData) {
            logger.warn({
                message: "Routes dynamic field does not contain a `data` field."
            });
        } else if (!objectData.bcs) {
            logger.warn({
                message: "Routes dynamic field does not contain a `bcs` field."
            });
        } else if (!objectData.bcs.dataType) {
            logger.warn({
                message: "Routes dynamic field does not contain a `dataType` field."
            });
        }
        throw new Error("Routes object data could not be fetched.");
    }

    /**
     * Fetches the dynamic field object for routes.
     *
     * @param client - The SuiClient instance.
     * @param siteObjectId - The ID of the site object.
     * @returns The dynamic field object for routes.
     */
    private async fetchRoutesDynamicField(
        siteObjectId: string,
    ): Promise<SuiObjectResponse> {
        return await this.rpcSelector.getDynamicFieldObject({
            parentId: siteObjectId,
            name: { type: "vector<u8>", value: "routes" },
        });
    }

    /**
     * Fetches the routes object using the dynamic field object ID.
     *
     * @param client - The SuiClient instance.
     * @param objectId - The ID of the dynamic field object.
     * @returns The routes object.
     */
    private async fetchRoutesObject(objectId: string): Promise<SuiObjectResponse> {
        return await this.rpcSelector.getObject({
            id: objectId,
            options: { showBcs: true },
        });
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
        if (routes.routes_list.size == 0) {
            // If the map is empty there is no match.
            return undefined;
        }

        // TODO: improve this using radix trees.
        const res = Array.from(routes.routes_list.entries())
            .filter(([pattern, _]) => new RegExp(`^${pattern.replace("*", ".*")}$`).test(path))
            .reduce((a, b) => (a[0].length >= b[0].length ? a : b));

        return res ? res[1] : undefined;
    }
}
