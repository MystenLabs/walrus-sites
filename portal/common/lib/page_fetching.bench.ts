// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { bench, describe, expect } from 'vitest';
import { fetchPage } from './page_fetching';
import { SuiClient, getFullnodeUrl } from '@mysten/sui/client'
import { NETWORK } from './constants';

const LANDING_PAGE_OID = '0x5fa99da7c4af9e2e2d0fb4503b058b9181693e463998c87c40be78fa2a1ca271';
const FLATLAND_OID = '0x049b6d3f34789904efcc20254400b7dca5548ee35cd7b5b145a211f85b2532fa';
const ws_pages = [
    [LANDING_PAGE_OID, "landing page"],
    [FLATLAND_OID, "flatland"]
]
const rpcUrl = getFullnodeUrl(NETWORK);
const client = new SuiClient({ url: rpcUrl });
describe('Page fetching', () => {
    ws_pages.forEach(([objectId, siteName]) => {
        bench(`fetchResource: fetch the ${siteName} walrus site`, async () => {
            const resourcePath = '/index.html';
            const res = await fetchPage(client, objectId, resourcePath);
            expect(res.status).toEqual(200); // If this fails, then the bench will result in 0ms
        });
    })
});
