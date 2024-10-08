// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { getRoutes, matchPathToRoute } from "./routing";
import { describe, test, expect } from 'vitest';

const snakeSiteObjectId = '0x3e01b1b8bf0e54f7843596345faff146f1047e304410ed2eb85d5f67ad404206';
test.skip('getRoutes', async () => {
    // TODO: when you make sure get_routes fetches
    // the Routes dynamic field, mock the request.
    const routes = await getRoutes(snakeSiteObjectId);
    console.log(routes)
});

test('matchPathToRoute', () => {
    const routes = {
        routes_list: new Map<string, string>([
            ['/path/*', '/index.html']
        ])
    };
    const match = matchPathToRoute("/path/", routes)
    console.log('MATCH: ', match)
    expect(match).toEqual("/index.html")
})
