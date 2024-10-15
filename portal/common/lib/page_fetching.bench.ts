// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { describe, bench, expect, vi, beforeAll, beforeEach, afterAll } from 'vitest';
import { fetchPage } from './page_fetching';
import { SuiClient, SuiObjectData } from '@mysten/sui/client';
import { sha256 } from './crypto';
import { toBase64 } from '@mysten/bcs';
import { checkRedirect } from './redirects';
import { Resource } from './types';

// Mock content and expected hash.
const mockContent = '<html>Mock Page Content</html>';
const contentBuffer = Buffer.from(mockContent, 'utf8');
let expectedHash: string;

const fetchMock = vi.fn();

const getObject = vi.fn();

const mockClient = {
    getObject,
} as unknown as SuiClient;

vi.mock('./redirects', () => ({
    checkRedirect: vi.fn(),
}));

describe('Page fetching with mocked network calls', () => {
    beforeAll(async () => {
        globalThis.fetch = fetchMock;

        const decompressed = new Uint8Array(contentBuffer);
        const hashArray = await sha256(decompressed);
        expectedHash = toBase64(hashArray);

        fetchMock.mockResolvedValue({
            ok: true,
            status: 200,
            headers: new Headers([
                ['Content-Type', 'text/html'],
                ['Content-Encoding', 'utf8'],
            ]),
            arrayBuffer: async () => contentBuffer,
            text: async () => mockContent,
            json: async () => ({ message: 'Mock Page Content' }),
        } as unknown as Response);

        // Mock 'bcs_data_parsing'.
        vi.mock('./bcs_data_parsing', () => ({
            ResourcePathStruct: {},
            ResourceStruct: {},
            DynamicFieldStruct: () => ({
                parse: vi.fn().mockReturnValue({
                    value: {
                        blob_id: '0xresourceBlobId',
                        path: '/index.html',
                        blob_hash: expectedHash,
                        headers: new Map([
                            ['Content-Type', 'text/html'],
                            ['Content-Encoding', 'utf8'],
                        ]),
                    } as Resource
                }),
            }),
        }));
    });

    beforeEach(() => {
        // Clear mocks.
        getObject.mockClear();
    });

    afterAll(() => {
        // Restore mocks.
        delete globalThis.fetch;
        vi.restoreAllMocks();
    });

    const landingPageObjectId = '0x1';
    const flatlanderObjectId = '0x2';

    // 1. Benchmark for normal page fetching.
    bench.skip('fetchPage: should successfully fetch the mocked landing page site', async () => {
        getObject.mockResolvedValueOnce({
            data: {
                bcs: {
                    dataType: 'moveObject',
                    bcsBytes: 'mockBcsBytes',
                },
            } as SuiObjectData,
        });

        const response = await fetchPage(mockClient, landingPageObjectId, '/index.html');
        expect(response.status).toEqual(200);
    });

    // 2. Benchmark for page fetching with redirect.
    bench.skip('fetchPage: should successfully fetch a mocked page site using redirect',
        async () => {
        (checkRedirect as any)
            .mockResolvedValueOnce('0x3')
            .mockResolvedValueOnce(undefined);

        getObject
            .mockResolvedValueOnce({
                data: {
                    bcs: {
                        dataType: 'moveObject',
                        bcsBytes: 'mockBcsBytes',
                    },
                } as SuiObjectData,
            }).mockResolvedValueOnce({
                data: {
                    bcs: {
                        dataType: 'moveObject',
                        bcsBytes: 'mockBcsBytes',
                    },
                } as SuiObjectData,
            }).mockResolvedValueOnce({
                data: {
                    bcs: {
                        dataType: 'moveObject',
                        bcsBytes: 'mockBcsBytes',
                    },
                } as SuiObjectData,
            });

        const response = await fetchPage(mockClient, flatlanderObjectId, '/index.html');
        expect(checkRedirect).toHaveBeenCalledWith(mockClient, flatlanderObjectId);
        expect(response.status).toEqual(200);
    });

});
