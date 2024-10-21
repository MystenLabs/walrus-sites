// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

// Import necessary functions and types
import { beforeEach, describe, expect, test, vi } from "vitest";
import { fetchResource } from "./resource";
import { SuiClient } from "@mysten/sui/client";
import { HttpStatusCodes } from "./http/http_status_codes";
import { checkRedirect } from "./redirects";
import { fromBase64 } from "@mysten/bcs";

// Mock SuiClient methods
const multiGetObjects = vi.fn();
const mockClient = {
    multiGetObjects,
} as unknown as SuiClient;

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

        const result = await fetchResource(mockClient, "0xParentId", "/path", seenResources);
        expect(result).toBe(HttpStatusCodes.LOOP_DETECTED);
    });

    test("TOO_MANY_REDIRECTS if recursion depth exceeds MAX_REDIRECT_DEPTH", async () => {
        const seenResources = new Set<string>();
        // Assuming MAX_REDIRECT_DEPTH is 3
        const result = await fetchResource(mockClient, "0xParentId", "/path", seenResources, 4);
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
                    },
                },
            },
            {
                data: {
                    bcs: {
                        dataType: "moveObject",
                        bcsBytes: "mockBcsBytes",
                    },
                },
            },
        ]);
        (fromBase64 as any).mockReturnValueOnce("decodedBcsBytes");

        const result = await fetchResource(mockClient, "0x1", "/path", new Set());
        expect(result).toEqual({
            blob_id: "0xresourceBlobId",
            objectId: "0xdynamicFieldId",
            version: undefined,
        });
        expect(checkRedirect).toHaveBeenCalledTimes(1);
    });

    test("should follow redirect and recursively fetch resource", async () => {
        const mockObject = {
            "objectId": "0x26dc2460093a9d6d31b58cb0ed1e72b19d140542a49be7472a6f25d542cb5cc3",
            "version": "150835605",
            "digest": "DDD7ZZvLkBQjq1kJpRsPDMpqhvYtGM878SdCTfF42ywE",
            "display": {
                "data": {
                    "walrus site address": "0x2"
                },
                "error": null
            },
            "bcs": {
                "dataType": "moveObject",
                "type": "0x1::flatland::Flatlander",
                "hasPublicTransfer": true,
                "version": 150835605,
            }
        };

        const mockResource = {
            data: {
                bcs: {
                    dataType: "moveObject",
                    bcsBytes: "mockBcsBytes",
                },
            },
        };

        (checkRedirect as any).mockResolvedValueOnce(undefined);

        multiGetObjects
            .mockResolvedValueOnce([mockObject, mockResource])
            .mockResolvedValueOnce([mockObject, mockResource]);

        const result = await fetchResource(
            mockClient,
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
        multiGetObjects.mockReturnValue([
            {
                data: {
                    bcs: {
                        dataType: "moveObject",
                    },
                },
            },
            {},
        ]);
        (fromBase64 as any).mockReturnValueOnce(undefined);

        const result = await fetchResource(mockClient, "0x1", "/path", seenResources);

        // Since the resource does not have a blob_id, the function should return NOT_FOUND
        expect(result).toBe(HttpStatusCodes.NOT_FOUND);
    });

    test("should return NOT_FOUND if dynamic fields are not found", async () => {
        const seenResources = new Set<string>();

        // Mock to return no redirect
        (checkRedirect as any).mockResolvedValueOnce(null);

        // Mock to simulate that dynamic fields are not found
        multiGetObjects.mockReturnValue([
            {data: {bcs: {dataType: "moveObject"}}},
            {},
        ]);

        const result = await fetchResource(mockClient, "0x1", "/path", seenResources);

        // Check that the function returns NOT_FOUND
        expect(result).toBe(HttpStatusCodes.NOT_FOUND);
    });
});
