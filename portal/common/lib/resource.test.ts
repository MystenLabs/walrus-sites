// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

// Import necessary functions and types
import { beforeEach, describe, expect, test, vi } from 'vitest';
import { fetchResource } from './resource';
import { SuiClient } from '@mysten/sui/client';
import { HttpStatusCodes } from './http/http_status_codes';
import { checkRedirect } from './redirects';
import { fromB64 } from '@mysten/bcs';
import { DynamicFieldStruct } from './bcs_data_parsing';
import { RESOURCE_PATH_MOVE_TYPE } from './constants';

// Mock SuiClient methods
const getDynamicFieldObject = vi.fn();
const getObject = vi.fn();

const mockClient = {
    getDynamicFieldObject,
    getObject,
} as unknown as SuiClient;

// Mock checkRedirect
vi.mock('./redirects', () => ({
    checkRedirect: vi.fn(),
}));

// Mock fromB64
vi.mock('@mysten/bcs', async () => {
    const actual = await vi.importActual<typeof import('@mysten/bcs')>('@mysten/bcs');
    return {
        ...actual,
        fromB64: vi.fn(),
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

describe('fetchResource', () => {
    beforeEach(() => {
        vi.clearAllMocks();
    });

    test('should return LOOP_DETECTED if objectId is already in seenResources', async () => {
        const seenResources = new Set<string>(['0xParentId']);

        const result = await fetchResource(mockClient, '0xParentId', '/path', seenResources);
        expect(result).toBe(HttpStatusCodes.LOOP_DETECTED);
    });

    test('should return TOO_MANY_REDIRECTS if recursion depth exceeds MAX_REDIRECT_DEPTH',
        async () => {
            const seenResources = new Set<string>();
            // Assuming MAX_REDIRECT_DEPTH is 3
            const result = await fetchResource(mockClient, '0xParentId', '/path', seenResources, 4);
            expect(result).toBe(HttpStatusCodes.TOO_MANY_REDIRECTS);
        });

    test('should fetch resource without redirect', async () => {
        // Mock dynamic field response
        getDynamicFieldObject.mockResolvedValueOnce({
            data: {
                objectId: '0xObjectId',
            },
        });
        // Mock object response
        getObject.mockResolvedValueOnce({
            data: {
                bcs: {
                    dataType: 'moveObject',
                    bcsBytes: 'mockBcsBytes',
                },
            },
        });
        (fromB64 as any).mockReturnValueOnce('decodedBcsBytes');

        const result = await fetchResource(mockClient, '0xParentId', '/path', new Set());

        expect(result).toEqual({
            blob_id: '0xresourceBlobId', objectId: '0xObjectId', version: undefined
        });
        expect(mockClient.getDynamicFieldObject).toHaveBeenCalledWith({
            parentId: '0xParentId',
            name: { type: RESOURCE_PATH_MOVE_TYPE, value: '/path' },
        });
        expect(mockClient.getObject).toHaveBeenCalledWith({
            id: '0xObjectId',
            options: { showBcs: true },
        });
    });

    test('should follow redirect and recursively fetch resource', async () => {
        // Mock dynamic field response for the initial object
        getDynamicFieldObject.mockResolvedValueOnce(null);

        // Mock the redirect check to return a redirect ID on the first call
        (checkRedirect as any).mockResolvedValueOnce('0xRedirectId');

        // Mock dynamic field response for the redirected object
        getDynamicFieldObject.mockResolvedValueOnce({
            data: {
                objectId: '0xFinalObjectId',
            },
        });

        // Mock the final resource object response
        getObject.mockResolvedValueOnce({
            data: {
                bcs: {
                    dataType: 'moveObject',
                    bcsBytes: 'mockBcsBytes',
                },
            },
        });

        const result = await fetchResource(mockClient, '0xParentId', '/path', new Set());

        // Verify the results
        expect(result).toEqual({
            blob_id: '0xresourceBlobId', objectId: '0xFinalObjectId', version: undefined
        });

        // Verify the correct sequence of calls

        // Initial redirect check and dynamic field fetch
        expect(checkRedirect).toHaveBeenNthCalledWith(1, mockClient, '0xParentId');
        expect(mockClient.getDynamicFieldObject).toHaveBeenNthCalledWith(1, {
            parentId: '0xParentId',
            name: { type: RESOURCE_PATH_MOVE_TYPE, value: '/path' },
        });

        expect(mockClient.getDynamicFieldObject).toHaveBeenNthCalledWith(2, {
            parentId: '0xRedirectId',
            name: { type: RESOURCE_PATH_MOVE_TYPE, value: '/path' },
        });

        // Final resource fetch after resolving the redirect
        expect(mockClient.getObject).toHaveBeenNthCalledWith(1, {
            id: '0xFinalObjectId',
            options: { showBcs: true },
        });
    });

    test('should return NOT_FOUND if the resource does not contain a blob_id', async () => {
        const seenResources = new Set<string>();
        const mockResource = {};  // No blob_id

        // Mock getDynamicFieldObject to return a valid object ID
        getDynamicFieldObject.mockResolvedValueOnce({
            data: { objectId: '0xObjectId' },
        });

        // Mock getObject to return a valid BCS object
        getObject.mockResolvedValueOnce({
            data: {
                bcs: {
                    dataType: 'moveObject',
                    bcsBytes: 'mockBcsBytes',
                },
            },
        });

        // Mock fromB64 to simulate the decoding process
        (fromB64 as any).mockReturnValueOnce('decodedBcsBytes');

        // Mock DynamicFieldStruct to return a resource without a blob_id
        (DynamicFieldStruct as any).mockImplementation(() => ({
            parse: () => ({ value: mockResource }),
        }));

        const result = await fetchResource(mockClient, '0xParentId', '/path', seenResources);

        // Since the resource does not have a blob_id, the function should return NOT_FOUND
        expect(result).toBe(HttpStatusCodes.NOT_FOUND);
    });


    test('should return NOT_FOUND if dynamic fields are not found', async () => {
        const seenResources = new Set<string>();

        // Mock to return no redirect
        (checkRedirect as any).mockResolvedValueOnce(null);

        // Mock to simulate that dynamic fields are not found
        getDynamicFieldObject.mockResolvedValueOnce({ data: null });

        const result = await fetchResource(mockClient, '0xParentId', '/path', seenResources);

        // Check that the function returns NOT_FOUND
        expect(result).toBe(HttpStatusCodes.NOT_FOUND);
    });

    test('should correctly handle a chain of redirects', async () => {
        const seenResources = new Set<string>();
        const mockResource = { blob_id: '0xresourceBlobId' };

        // First redirect: no dynamic fields and checkRedirect yields an objectId.
        getDynamicFieldObject
            .mockResolvedValueOnce(null)
            .mockResolvedValueOnce(null)
            .mockResolvedValueOnce({
                data: { objectId: '0xFinalObjectId' },
            });
        (checkRedirect as any)
            .mockResolvedValueOnce('0xredirect1')
            .mockResolvedValueOnce('0xredirect2');

        // Mock getObject to return a valid response for each object in the chain
        getObject.mockResolvedValueOnce({
            data: {
                bcs: { dataType: 'moveObject', bcsBytes: 'mockBcsBytes' },
            },
        });

        // Mock fromB64 to simulate the decoding process
        (fromB64 as any).mockReturnValueOnce('decodedBcsBytes');

        // Mock DynamicFieldStruct to parse the BCS data and return the mock resource
        (DynamicFieldStruct as any).mockImplementation(() => ({
            parse: () => ({ value: mockResource }),
        }));

        const result = await fetchResource(mockClient, '0xParentId', '/path', seenResources);

        expect(checkRedirect).toHaveBeenNthCalledWith(1, mockClient, '0xParentId');
        expect(checkRedirect).toHaveBeenNthCalledWith(2, mockClient, '0xredirect1');
        // Validate the correct resource is returned after following the chain of redirects
        expect(result).toEqual({
            blob_id: '0xresourceBlobId',
            objectId: '0xFinalObjectId',
            version: undefined
        });
    });
});
