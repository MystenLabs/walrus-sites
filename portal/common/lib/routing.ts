// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { getFullnodeUrl, SuiClient, SuiObjectResponse } from "@mysten/sui/client";
import { Routes } from "./types";
import { DynamicFieldStruct, RoutesStruct } from "./bcs_data_parsing";
import { bcs, fromBase64 } from "@mysten/bcs";

/**
 * Gets the Routes dynamic field of the site object.
 * Returns the extracted routes_list map to use for future requests,
 * and redirects the paths matched accordingly.
 *
 * @param siteObjectId - The ID of the site object.
 * @returns The routes list.
 */
export async function getRoutes(
    client: SuiClient,
    siteObjectId: string,
): Promise<Routes | undefined> {
    const routesDF = await fetchRoutesDynamicField(client, siteObjectId);
    if (!routesDF.data) {
        console.warn("No routes dynamic field found for site object.");
        return;
    }
    const routesObj = await fetchRoutesObject(client, routesDF.data.objectId);
    const objectData = routesObj.data;
    if (objectData && objectData.bcs && objectData.bcs.dataType === "moveObject") {
        return parseRoutesData(objectData.bcs.bcsBytes);
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
async function fetchRoutesDynamicField(
    client: SuiClient,
    siteObjectId: string,
): Promise<SuiObjectResponse> {
    return await client.getDynamicFieldObject({
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
async function fetchRoutesObject(client: SuiClient, objectId: string): Promise<SuiObjectResponse> {
    return await client.getObject({
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
function parseRoutesData(bcsBytes: string): Routes {
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
export function matchPathToRoute(path: string, routes: Routes): string | undefined {
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
