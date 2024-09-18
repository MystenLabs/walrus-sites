// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { bench, describe, expect } from 'vitest';
import { fetchPage } from './page_fetching';
import { SuiClient, getFullnodeUrl } from '@mysten/sui/client'
import { NETWORK } from './constants';
import { SITES_USED_FOR_BENCHING } from './constants';

const rpcUrl = getFullnodeUrl(NETWORK);
const client = new SuiClient({ url: rpcUrl });
describe('Page fetching', () => {
    SITES_USED_FOR_BENCHING.forEach(([objectId, siteName]) => {
        bench(`fetchPage: fetch the ${siteName} walrus site`, async () => {
            const resourcePath = '/index.html';
            const res = await fetchPage(client, objectId, resourcePath);
            expect(res.status).toEqual(200); // If this fails, then the bench will result in 0ms
        });
    })
});
