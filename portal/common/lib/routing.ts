// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { getFullnodeUrl, SuiClient, SuiObjectData } from "@mysten/sui/client";
import { NETWORK } from "./constants";
import { Routes, } from "./types";
import {
    DynamicFieldStruct,
    RoutesStruct,
} from "./bcs_data_parsing";
import { bcs, BcsType, fromB64 } from "@mysten/bcs";


/**
 * Gets the Routes dynamic field of the site object.
 * Returns the extracte the routes_list map in order
 * to use it for future requests, and redirect the
 * paths matched accordingly.
 */
export async function getRoutes(siteObjectId: string) {
    const rpcUrl = getFullnodeUrl(NETWORK);
    const client = new SuiClient({ url: rpcUrl });

    const routesDF = await client.getDynamicFieldObject({
        parentId: siteObjectId,
        name: { type: "vector<u8>", value: "routes" },
    });

    const routesObj = await client.getObject({
        id: routesDF.data.objectId,
        options: { showBcs: true }
    })

    const objectData = routesObj.data;
    if (objectData && objectData.bcs.dataType === "moveObject") {
        const df = DynamicFieldStruct(
                bcs.vector(bcs.u8()),
                RoutesStruct
            ).parse(
                fromB64(objectData.bcs.bcsBytes)
            );
        return df.value;
    }
    throw new Error("Could not parse routes DF object.");
}

/**
 * Matches the path to the appropriate route.
 * Returns the path of the matched route.
 * @param path The path to match.
 * @param routes The routes to match against.
 */
export function matchRoutes(path: string, routes: Routes): string {
    return "TODO"
}
