// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { bench, describe, expect, vi, beforeEach } from 'vitest';
import { fetchResource } from './resource';
import { SuiClient, SuiObjectData } from '@mysten/sui/client';
import { isVersionedResource } from './types';
import { SITES_USED_FOR_BENCHING } from './constants';

// Mock SuiClient methods.
const getDynamicFieldObject = vi.fn();
const getObject = vi.fn();

const mockClient = {
    getDynamicFieldObject,
    getObject,
} as unknown as SuiClient;

vi.mock('./bcs_data_parsing', () => ({
    ResourcePathStruct: vi.fn(),
    ResourceStruct: vi.fn(),
    DynamicFieldStruct: vi.fn(() => ({
        parse: vi.fn().mockReturnValue({
            value: {
                blob_id: '0xresourceBlobId',
                path: '/index.html',
                content_type: 'text/html',
                content_encoding: 'utf8',
                blob_hash: 'mockedBlobHash',
            },
        }),
    })),
}));

describe('Resource fetching with mocked network calls', () => {
    beforeEach(() => {
        vi.restoreAllMocks();

        // Mock `getDynamicFieldObject` to provide data.
        getDynamicFieldObject.mockResolvedValue({
            data: {
                objectId: '0xObjectId',
                version: '1',
                digest: 'mocked-digest',
            },
        });

        // Mock `getObject` to provide BCS data.
        getObject.mockResolvedValue({
            data: {
                objectId: '0xObjectId',
                version: '1',
                digest: 'mocked-digest',
                bcs: {
                    dataType: 'moveObject',
                    bcsBytes: Buffer.from('valid-mocked-bcs-data', 'utf8').toString('base64'),
                },
            } as SuiObjectData,
        });
    });

    SITES_USED_FOR_BENCHING.forEach(([objectId, siteName]) => {
        // Benchmark the performance of fetching resources.
        bench(`fetchResource: fetch the ${siteName} site`, async () => {
            const resourcePath = '/index.html';

            // Use the mocked client and call fetchResource.
            const resp = await fetchResource(mockClient, objectId, resourcePath, new Set());

            // Validate that the fetched resource is versioned.
            expect(isVersionedResource(resp)).toBeTruthy();
        });
    });
});
