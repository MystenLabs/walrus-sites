// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { bench, describe, expect } from 'vitest';
import { fetchResource } from './resource';
import { SuiClient, getFullnodeUrl } from '@mysten/sui/client'
import { NETWORK } from './constants';
import { isVersionedResource } from './types';
import { SITES_USED_FOR_BENCHING } from './constants';

const rpcUrl = getFullnodeUrl(NETWORK);
const client = new SuiClient({ url: rpcUrl });
describe('Resource fetching', () => {
    SITES_USED_FOR_BENCHING.forEach(([objectId, siteName]) => {
        bench(`fetchResource: fetch the ${siteName} walrus site`, async () => {
            const resourcePath = '/index.html';
            const resp = await fetchResource(client, objectId, resourcePath, new Set());
            expect(isVersionedResource(resp)).toBeTruthy();
        });
    })
});
