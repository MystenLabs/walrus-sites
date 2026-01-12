// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

// Import necessary functions and types
import { beforeEach, describe, expect, test, vi } from "vitest";
import { ResourceFetcher } from "@lib/resource";
import { RPCSelector } from "@lib/rpc_selector";
import { HttpStatusCodes } from "@lib/http/http_status_codes";
import { checkRedirect } from "@lib/redirects";
import { fromBase64 } from "@mysten/bcs";
import { SuiObjectResponse } from "@mysten/sui/client";


vi.mock("@mysten/sui/utils", () => ({
    deriveDynamicFieldID: vi.fn(() => "0xdynamicFieldId"),
}))

// Mock checkRedirect
vi.mock("../src/redirects", () => ({
    checkRedirect: vi.fn(),
}));

// Mock fromBase64
vi.mock("@mysten/bcs", async () => {
    const actual = await vi.importActual<typeof import("@mysten/bcs")>("@mysten/bcs");
    return {
        ...actual,
        fromBase64: vi.fn(),
    };
});

vi.mock("../src/bcs_data_parsing", async (importOriginal) => {
    const actual = (await importOriginal()) as typeof import("../src/bcs_data_parsing");
    return {
        ...actual,
        DynamicFieldStruct: vi.fn(() => ({
            parse: vi.fn(() => ({ value: { blob_id: "0xresourceBlobId" } })),
        })),
    };
});

describe("fetchResource", () => {
    console.log("RPC URLS:", process.env.RPC_URL_LIST!.split(','));
    const rpcSelector = new RPCSelector(
        process.env.RPC_URL_LIST!.split(','),
        'testnet'
    )
    const resourceFetcher = new ResourceFetcher(rpcSelector, "0x123");
    // Mock SuiClient methods
    const multiGetObjects = vi.spyOn(rpcSelector, 'multiGetObjects');

    beforeEach(() => {
        vi.clearAllMocks();
    });

    test("should return LOOP_DETECTED if objectId is already in seenResources", async () => {
        const seenResources = new Set<string>(["0xParentId"]);

        const result = await resourceFetcher.fetchResource("0xParentId", "/path", seenResources);
        expect(result).toBe(HttpStatusCodes.LOOP_DETECTED);
    });

    test("TOO_MANY_REDIRECTS if recursion depth exceeds MAX_REDIRECT_DEPTH", async () => {
        const seenResources = new Set<string>();
        // Assuming MAX_REDIRECT_DEPTH is 3
        const result = await resourceFetcher.fetchResource("0xParentId", "/path", seenResources, 4);
        expect(result).toBe(HttpStatusCodes.TOO_MANY_REDIRECTS);
    });

    test("should fetch resource without redirect", async () => {

        // Mock object response
        multiGetObjects.mockResolvedValueOnce([
            {
                data: {
                    bcs: {
                        dataType: "moveObject",
                        bcsBytes: "mockBcsBytes",
                        hasPublicTransfer: true,
                        type: "mockType",
                        version: undefined
                    },
                    digest: "",
                    objectId: "",
                    version: undefined
                },
            },
            {
                data: {
                    bcs: {
                        dataType: "moveObject",
                        bcsBytes: "mockBcsBytes",
                        hasPublicTransfer: true,
                        type: "mockType",
                        version: "1.0"
                    },
                    digest: "",
                    objectId: "",
                    version: undefined
                },
            },
        ]);
        (fromBase64 as any).mockReturnValueOnce("decodedBcsBytes");

        const result = await resourceFetcher.fetchResource("0x1", "/path", new Set());
        expect(result).toEqual({
            blob_id: "0xresourceBlobId",
            objectId: "0xdynamicFieldId",
            version: undefined,
        });
        expect(checkRedirect).toHaveBeenCalledTimes(1);
    });

    test("should follow redirect and recursively fetch resource", async () => {
        const mockObject: SuiObjectResponse = {
            data: {
                bcs: {
                    dataType: "moveObject",
                    bcsBytes: "mockBcsBytes",
                    hasPublicTransfer: true,
                    type: "mockType",
                    version: undefined
                },
                display: {
                    data: {
                        "walrus site address": "mockAddress",
                    },
                    error: null,
                },
                digest: "",
                objectId: "",
                version: undefined
            }
        };

        const mockResource: SuiObjectResponse = {
            data: {
                bcs: {
                    dataType: "moveObject",
                    bcsBytes: "mockBcsBytes",
                    hasPublicTransfer: true,
                    type: "mockType",
                    version: undefined,
                },
                display: {
                    data: {
                        "walrus site address": "mockAddress",
                    },
                    error: null,
                },
                digest: "",
                objectId: "",
                version: undefined
            }
        };


        (checkRedirect as any).mockResolvedValueOnce(undefined);

        multiGetObjects
            .mockResolvedValueOnce([mockObject, mockResource])
            .mockResolvedValueOnce([mockObject, mockResource]);

        const result = await resourceFetcher.fetchResource(
            "0x26dc2460093a9d6d31b58cb0ed1e72b19d140542a49be7472a6f25d542cb5cc3",
            "/path",
            new Set());

        // Verify the results
        expect(result).toEqual({
            blob_id: "0xresourceBlobId",
            objectId: "0xdynamicFieldId",
            version: undefined,
        });
        expect(checkRedirect).toHaveBeenCalledTimes(2);
    });

    test("should return NOT_FOUND if the resource does not contain a blob_id", async () => {
        const seenResources = new Set<string>();
        (checkRedirect as any).mockResolvedValueOnce(undefined);
        multiGetObjects.mockResolvedValue([
            {
                data: {
                    bcs: {
                        dataType: "moveObject",
                        bcsBytes: "mockBcsBytes",
                        hasPublicTransfer: true,
                        type: "mockType",
                        version: "1.0"
                    },
                digest: "",
                objectId: "",
                version: "1.0"
                }
            },
            {},
        ]);
        (fromBase64 as any).mockReturnValueOnce(undefined);

        const result = await resourceFetcher.fetchResource("0x1", "/path", seenResources);

        // Since the resource does not have a blob_id, the function should return NOT_FOUND
        expect(result).toBe(HttpStatusCodes.NOT_FOUND);
    });

    test("should return NOT_FOUND if dynamic fields are not found", async () => {
        const seenResources = new Set<string>();

        // Mock to return no redirect
        (checkRedirect as any).mockResolvedValueOnce(null);

        // Mock to simulate that dynamic fields are not found
        multiGetObjects.mockResolvedValue([
            {
                data: {
                    bcs: {
                        dataType: "moveObject",
                        bcsBytes: "mockBcsBytes",
                        hasPublicTransfer: true,
                        type: "mockType",
                        version: "1.0"
                    },
                    digest: "mockDigest",
                    objectId: "mockObjectId",
                    version: "1.0"
                }
            },
            {}
        ]);

        const result = await resourceFetcher.fetchResource("0x1", "/path", seenResources);

        // Check that the function returns NOT_FOUND
        expect(result).toBe(HttpStatusCodes.NOT_FOUND);
    });
});
