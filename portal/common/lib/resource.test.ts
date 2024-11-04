// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

// Import necessary functions and types
import { beforeEach, describe, expect, test, vi } from "vitest";
import { fetchResource } from "./resource";
import { HttpStatusCodes } from "./http/http_status_codes";
import { checkRedirect } from "./redirects";
import { fromBase64 } from "@mysten/bcs";
import rpcSelectorSingleton from "./rpc_selector";
import { SuiObjectResponse } from "@mysten/sui/client";

// Mock SuiClient methods
const multiGetObjects = vi.spyOn(rpcSelectorSingleton, 'multiGetObjects');

vi.mock("@mysten/sui/utils", () => ({
    deriveDynamicFieldID: vi.fn(() => "0xdynamicFieldId"),
}))

// Mock checkRedirect
vi.mock("./redirects", () => ({
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

vi.mock("./bcs_data_parsing", async (importOriginal) => {
    const actual = (await importOriginal()) as typeof import("./bcs_data_parsing");
    return {
        ...actual,
        DynamicFieldStruct: vi.fn(() => ({
            parse: vi.fn(() => ({ value: { blob_id: "0xresourceBlobId" } })),
        })),
    };
});

describe("fetchResource", () => {
    beforeEach(() => {
        vi.clearAllMocks();
    });

    test("should return LOOP_DETECTED if objectId is already in seenResources", async () => {
        const seenResources = new Set<string>(["0xParentId"]);

        const result = await fetchResource("0xParentId", "/path", seenResources);
        expect(result).toBe(HttpStatusCodes.LOOP_DETECTED);
    });

    test("TOO_MANY_REDIRECTS if recursion depth exceeds MAX_REDIRECT_DEPTH", async () => {
        const seenResources = new Set<string>();
        // Assuming MAX_REDIRECT_DEPTH is 3
        const result = await fetchResource("0xParentId", "/path", seenResources, 4);
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

        const result = await fetchResource("0x1", "/path", new Set());
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

        const result = await fetchResource(
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

        const result = await fetchResource("0x1", "/path", seenResources);

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

        const result = await fetchResource("0x1", "/path", seenResources);

        // Check that the function returns NOT_FOUND
        expect(result).toBe(HttpStatusCodes.NOT_FOUND);
    });
});
