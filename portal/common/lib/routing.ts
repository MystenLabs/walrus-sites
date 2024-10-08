// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { getFullnodeUrl, SuiClient } from "@mysten/sui/client";
import { NETWORK } from "./constants";
import { Routes, } from "./types";
import {
    DynamicFieldStruct,
    RoutesStruct,
} from "./bcs_data_parsing";
import { bcs, fromB64 } from "@mysten/bcs";


/**
 * Gets the Routes dynamic field of the site object.
 * Returns the extracte the routes_list map in order
 * to use it for future requests, and redirect the
 * paths matched accordingly.
 */
export async function getRoutes(siteObjectId: string): Promise<Routes> {
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
                // BCS declaration of the ROUTES_FIELD in site.move.
                bcs.vector(bcs.u8()),
                // The value of the df, i.e. the Routes Struct.
                RoutesStruct
            ).parse(
                fromB64(objectData.bcs.bcsBytes)
            );
        return df.value as any as Routes;
    }
    throw new Error("Could not parse routes DF object.");
}

/**
 * Matches the path to the appropriate route.
 * Returns the path of the matched route.
 * @param path The path to match.
 * @param routes The routes to match against.
 */
export function matchPathToRoute(path: string, routes: Routes): string | undefined {
    const routesArraySorted: Array<[string, string]> = Array.from(
        routes.routes_list.entries()
    ).sort((current, next) => next[0].length - current[0].length);
    const res = routesArraySorted.find(
        ([pattern, _]) => new RegExp(`^${pattern.replace('*', '.*')}$`).test(path)
    );
    return res? res[1] : undefined
}
