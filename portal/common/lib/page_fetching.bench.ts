// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { describe, bench, expect, vi, beforeAll, beforeEach, afterAll } from 'vitest';
import { fetchPage } from './page_fetching';
import { SuiClient, SuiObjectData } from '@mysten/sui/client';
import { SITES_USED_FOR_BENCHING } from './constants';
import { sha256 } from './crypto';
import { toHEX } from '@mysten/bcs';

// Mock content and expected hash.
const mockContent = '<html>Mock Page Content</html>';
const contentBuffer = Buffer.from(mockContent, 'utf8');
let expectedHash: string;

// Mock functions.
const fetchMock = vi.fn();
const getDynamicFieldObject = vi.fn();
const getObject = vi.fn();
const mockClient = {
    getDynamicFieldObject,
    getObject,
} as unknown as SuiClient;

describe('Page fetching with mocked network calls', () => {
    beforeAll(async () => {
        // Set up the global fetch mock.
        globalThis.fetch = fetchMock;

        // Compute expected hash.
        const decompressed = new Uint8Array(contentBuffer);
        const hashArray = await sha256(decompressed);
        expectedHash = toHEX(hashArray);

        // Mock fetch response.
        fetchMock.mockResolvedValue({
            ok: true,
            status: 200,
            headers: new Headers({
                'Content-Type': 'text/html',
            }),
            arrayBuffer: async () => contentBuffer,
            text: async () => mockContent,
            json: async () => ({ message: 'Mock Page Content' }),
        } as unknown as Response);

        // Mock 'decompress_data'.
        vi.mock('./decompress_data', () => ({
            decompressData: vi.fn(async (data: Uint8Array) => data),
        }));

        // Mock 'bcs_data_parsing'.
        vi.mock('./bcs_data_parsing', () => ({
            ResourcePathStruct: {},
            ResourceStruct: {},
            DynamicFieldStruct: () => ({
                parse: vi.fn().mockReturnValue({
                    value: {
                        blob_id: '0xresourceBlobId',
                        path: '/index.html',
                        content_type: 'text/html',
                        content_encoding: 'utf8',
                        blob_hash: expectedHash,
                    },
                }),
            }),
        }));
    });

    beforeEach(() => {
        // Clear mocks.
        fetchMock.mockClear();
        getDynamicFieldObject.mockClear();
        getObject.mockClear();

        // Mock 'getDynamicFieldObject'.
        getDynamicFieldObject.mockResolvedValue({
            data: {
                objectId: '0xObjectId',
                version: '1',
                digest: 'mocked-digest',
            },
        });

        // Mock 'getObject'.
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

    afterAll(() => {
        // Restore mocks.
        delete globalThis.fetch;
        vi.restoreAllMocks();
    });

    SITES_USED_FOR_BENCHING.forEach(([objectId, siteName]) => {
        bench(`fetchPage: should successfully fetch the ${siteName} site`, async () => {
            const resourcePath = '/index.html';
            const response = await fetchPage(mockClient, objectId, resourcePath);

            expect(response.status).toEqual(200);
        });

        bench(`fetchPage: should return 404 for non-existing ${siteName} page`, async () => {
            // Mock fetch to return 404.
            fetchMock.mockResolvedValueOnce({
                ok: false,
                status: 404,
                headers: new Headers({
                    'Content-Type': 'text/plain',
                }),
                arrayBuffer: async () => Buffer.from('Not Found', 'utf8'),
                text: async () => 'Not Found',
                json: async () => ({ error: 'Not Found' }),
            } as unknown as Response);

            const resourcePath = '/non-existing.html';
            const response = await fetchPage(mockClient, objectId, resourcePath);

            expect(response.status).toEqual(404);
        });
    });
});
