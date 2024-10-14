// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { getRoutes, matchPathToRoute } from "./routing";
import { test, expect } from "vitest";
import { SuiClient, getFullnodeUrl } from "@mysten/sui/client";
import { NETWORK } from "./constants";

const snakeSiteObjectId = "0x7a95e4be3948415b852fb287d455166a276d7a52f1a567b4a26b6b5e9c753158";
test.skip("getRoutes", async () => {
    // TODO: when you make sure get_routes fetches
    // the Routes dynamic field, mock the request.
    const rpcUrl = getFullnodeUrl(NETWORK);
    const client = new SuiClient({ url: rpcUrl });
    const routes = await getRoutes(client, snakeSiteObjectId);
    console.log(routes);
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
        const match = matchPathToRoute(requestPath, routesExample);
        expect(match).toEqual(expected);
    });
});

// Test in the case there are no routes.
const emptyRoutes = { routes_list: new Map<string, string>() };

testCases.forEach(([requestPath, _]) => {
    test(`matchPathToRoute: empty routes for "${requestPath}"`, () => {
        const match = matchPathToRoute(requestPath, emptyRoutes);
        expect(match).toEqual(undefined);
    });
});
