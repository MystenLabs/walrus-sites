// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { bench, describe, expect, vi, beforeEach } from 'vitest';
import { fetchResource } from './resource';
import { SuiClient, SuiObjectData } from '@mysten/sui/client';
import { isVersionedResource } from './types';
import { checkRedirect } from './redirects';

// Mock SuiClient methods.
const getDynamicFieldObject = vi.fn();
const getObject = vi.fn();
const mockClient = {
    getDynamicFieldObject,
    getObject,
} as unknown as SuiClient;
// Mock `checkRedirect`.
vi.mock('./redirects', () => ({
    checkRedirect: vi.fn(),
}));
// Mock `bcs_data_parsing` to simulate parsing of the BCS data.
vi.mock('./bcs_data_parsing', () => ({
    ResourcePathStruct: vi.fn(),
    ResourceStruct: vi.fn(),
    DynamicFieldStruct: vi.fn(() => ({
        parse: vi.fn().mockReturnValue({
            value: {
                blob_id: '0xresourceBlobId',
                path: '/index.html',
                blob_hash: 'mockedBlobHash',
                headers: [
                    ['Content-Type', 'text/html'],
                    ['Content-Encoding', 'utf8'],
                ],
            },
        }),
    })),
}));

describe('Resource fetching with mocked network calls', () => {
    const landingPageObjectId = '0xLandingPage';
    const flatlanderObjectId = '0xFlatlanderObject';

    beforeEach(() => {
        getDynamicFieldObject.mockClear();
        getObject.mockClear();
        (checkRedirect as any).mockClear();
    });

    // 1. Benchmark for a page like the landing page (without redirects).
    bench('fetchResource: fetch the landing page site (no redirects)', async () => {
        const resourcePath = '/index.html';
        getDynamicFieldObject.mockResolvedValueOnce({
            data: {
                objectId: '0xObjectId',
                digest: 'mocked-digest',
            },
        });

        getObject.mockResolvedValueOnce({
            data: {
                bcs: {
                    dataType: 'moveObject',
                    bcsBytes: 'mockBcsBytes',
                },
            } as SuiObjectData,
        });

        const resp = await fetchResource(mockClient, landingPageObjectId, resourcePath, new Set());
        expect(isVersionedResource(resp)).toBeTruthy();
    });

    // 2. Benchmark for a page with redirects (such as accessing a Flatlander).
    bench('fetchResource: fetch the flatlander site (with redirects)', async () => {
        const resourcePath = '/index.html';

        getDynamicFieldObject.mockResolvedValueOnce(null);

        (checkRedirect as any).mockResolvedValueOnce('0xRedirectId');

        getDynamicFieldObject.mockResolvedValueOnce({
            data: {
                objectId: '0xFinalObjectId',
                digest: 'mocked-digest',
            },
        });

        getObject.mockResolvedValueOnce({
            data: {
                bcs: {
                    dataType: 'moveObject',
                    bcsBytes: 'mockBcsBytes',
                },
            } as SuiObjectData,
        });

        const resp = await fetchResource(mockClient, flatlanderObjectId, resourcePath, new Set());
        expect(isVersionedResource(resp)).toBeTruthy();
        expect(checkRedirect).toHaveBeenCalledWith(mockClient, flatlanderObjectId);
    });
});
