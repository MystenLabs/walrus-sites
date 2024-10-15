// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { bench, describe, expect, vi, beforeEach } from 'vitest';
import { fetchResource } from './resource';
import { SuiClient, SuiObjectData } from '@mysten/sui/client';
import { checkRedirect } from './redirects';
import { fromBase64 } from '@mysten/bcs';

const getObject = vi.fn();
const mockClient = {
    getObject,
} as unknown as SuiClient;

// Mock checkRedirect
vi.mock('./redirects', () => ({
    checkRedirect: vi.fn(),
}));

// Mock fromBase64
vi.mock('@mysten/bcs', async () => {
    const actual = await vi.importActual<typeof import('@mysten/bcs')>('@mysten/bcs');
    return {
        ...actual,
        fromBase64: vi.fn(),
    };
});

vi.mock('./bcs_data_parsing', async (importOriginal) => {
    const actual = await importOriginal() as typeof import('./bcs_data_parsing');
    return {
        ...actual,
        DynamicFieldStruct: vi.fn(() => ({
            parse: vi.fn(() => ({ value: { blob_id: '0xresourceBlobId' } })),
        })),
    };
});

describe('Resource fetching with mocked network calls', () => {
    const landingPageObjectId = '0x1';
    const flatlanderObjectId = '0x2';

    beforeEach(() => {
        getObject.mockClear();
        (checkRedirect as any).mockClear();
    });

    // 1. Benchmark for a page like the landing page (without redirects).
    bench('fetchResource: fetch the landing page site (no redirects)', async () => {
        // Mock object response
        getObject.mockResolvedValueOnce({
            data: {
                bcs: {
                    dataType: 'moveObject',
                    bcsBytes: 'mockBcsBytes',
                },
            },
        });
        (fromBase64 as any).mockReturnValueOnce('decodedBcsBytes');
        const resp = await fetchResource(mockClient, landingPageObjectId, '/index.html', new Set());
        expect(resp).toBeDefined();
    });

    // 2. Benchmark for a page with redirects (such as accessing a Flatlander).
    bench('fetchResource: fetch the flatlander site (with redirects)', async () => {
        const resourcePath = '/index.html';

        (checkRedirect as any)
            .mockResolvedValueOnce('0x3')
            .mockResolvedValueOnce(undefined);

        // Redirecting to the flatlander display object.
        getObject.mockResolvedValueOnce({
            data: {
                bcs: {
                    dataType: 'moveObject',
                    bcsBytes: 'mockBcsBytes',
                },
            } as SuiObjectData,
        });

        const resp = await fetchResource(mockClient, flatlanderObjectId, resourcePath, new Set());
        expect(checkRedirect).toHaveBeenCalledTimes(2);
        expect(resp).toBeDefined();
    });
});
