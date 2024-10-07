// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { getRoutes } from "./routing";
import { describe, it, expect } from 'vitest';

const snakeSiteObjectId = '0x3e01b1b8bf0e54f7843596345faff146f1047e304410ed2eb85d5f67ad404206';
describe('get_routes', () => {
    it('should return correct routes', async () => {
        // TODO: when you make sure get_routes fetches
        // the Routes dynamic field, mock the request.
        const routes = await getRoutes(snakeSiteObjectId);
        console.log(routes)
    });
});
