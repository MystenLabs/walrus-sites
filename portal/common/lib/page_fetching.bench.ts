// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { bench, describe, expect } from 'vitest';
import { fetchPage } from './page_fetching';
import { SuiClient, getFullnodeUrl } from '@mysten/sui/client'
import { NETWORK } from './constants';

describe('Page fetching', () => {
    bench('fetch the landing page walrus site', async () => {
        const rpcUrl = getFullnodeUrl(NETWORK);
        const client = new SuiClient({ url: rpcUrl });
        const objectId = '0x5fa99da7c4af9e2e2d0fb4503b058b9181693e463998c87c40be78fa2a1ca271';
        const resourcePath = '/index.html';
        const res = await fetchPage(client, objectId, resourcePath);
        expect(res.status).toEqual(200); // If this fails, then the bench will result in 0ms
    });
});
