// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { getRoutes, matchPathToRoute } from "./routing";
import { test, expect } from 'vitest';
import { SuiClient, getFullnodeUrl } from "@mysten/sui/client";
import { NETWORK } from "./constants";

const snakeSiteObjectId = '0x3e01b1b8bf0e54f7843596345faff146f1047e304410ed2eb85d5f67ad404206';
test.skip('getRoutes', async () => {
    // TODO: when you make sure get_routes fetches
    // the Routes dynamic field, mock the request.
    const rpcUrl = getFullnodeUrl(NETWORK);
    const client = new SuiClient({ url: rpcUrl });
    const routes = await getRoutes(client, snakeSiteObjectId);
    console.log(routes)
});

const routesExample = {
    routes_list: new Map<string, string>([
        ['/*', '/default.html'],
        ['/somewhere/else', 'else.jpeg'],
        ['/somewhere/else/*', 'star-else.gif'],
        ['/path/to/*', '/somewhere.html'],
    ])
};

const testCases = [
    ["/path/to/somewhere/", "/somewhere.html"],
    ["/somewhere/else", 'else.jpeg'],
    ["/", "/default.html"],
    ["/somewhere", "/default.html"],
    ["/somewhere/else/star", "star-else.gif"],
    ["/somewhere/else/", 'star-else.gif'],
]

testCases.forEach(([requestPath, expected]) => {
    test(`matchPathToRoute: "${requestPath}" -> "${expected}"`, () => {
        const match = matchPathToRoute(requestPath, routesExample)
        expect(match).toEqual(expected)
    })
});
