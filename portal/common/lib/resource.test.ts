// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

// Import necessary functions and types
import { beforeEach, describe, expect, test, vi } from "vitest";
import { fetchResource } from "./resource";
import { SuiClient } from "@mysten/sui/client";
import { HttpStatusCodes } from "./http/http_status_codes";
import { checkRedirect } from "./redirects";
import { fromBase64 } from "@mysten/bcs";
import { DynamicFieldStruct } from "./bcs_data_parsing";

// Mock SuiClient methods
const getObject = vi.fn();
const mockClient = {
    getObject,
} as unknown as SuiClient;

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
        getObject.mockResolvedValueOnce({
            data: {
                bcs: {
                    dataType: "moveObject",
                    bcsBytes: "mockBcsBytes",
                },
            },
        });
        (fromBase64 as any).mockReturnValueOnce("decodedBcsBytes");

        const result = await fetchResource(mockClient, "0x1", "/path", new Set());
        expect(result).toEqual({
            blob_id: "0xresourceBlobId",
            objectId: "0x3cf9bff169db6f780a0a3cae7b3b770097c26342ad0c08604bc80728cfa37bdc",
            version: undefined,
        });
        expect(mockClient.getObject).toHaveBeenCalledWith({
            id: "0x3cf9bff169db6f780a0a3cae7b3b770097c26342ad0c08604bc80728cfa37bdc",
            options: { showBcs: true },
        });
    });

    test("should follow redirect and recursively fetch resource", async () => {
        // Mock the redirect check to return a redirect ID on the first call
        (checkRedirect as any).mockResolvedValueOnce(
            "0x51813e7d4040265af8bd6c757f52accbe11e6df5b9cf3d6696a96e3f54fad096",
        );
        (checkRedirect as any).mockResolvedValueOnce(undefined);

        // Mock the first resource object response
        getObject.mockResolvedValueOnce({
            data: {
                bcs: {
                    dataType: "moveObject",
                    bcsBytes: "mockBcsBytes",
                },
            },
        });

        // Mock the final resource object response
        getObject.mockResolvedValueOnce({
            data: {
                bcs: {
                    dataType: "moveObject",
                    bcsBytes: "mockBcsBytes",
                },
            },
        });

        const result = await fetchResource(mockClient, "0x1", "/path", new Set());

        // Verify the results
        expect(result).toEqual({
            blob_id: "0xresourceBlobId",
            objectId: "0x3cf9bff169db6f780a0a3cae7b3b770097c26342ad0c08604bc80728cfa37bdc",
            version: undefined,
        });
        expect(checkRedirect).toHaveBeenCalledTimes(1);
    });

    test("should return NOT_FOUND if the resource does not contain a blob_id", async () => {
        const seenResources = new Set<string>();
        const mockResource = {}; // No blob_id

        (checkRedirect as any).mockResolvedValueOnce(
            "0x51813e7d4040265af8bd6c757f52accbe11e6df5b9cf3d6696a96e3f54fad096",
        );

        // Mock getObject to return a valid BCS object
        getObject.mockResolvedValueOnce({
            data: {
                bcs: {
                    dataType: "moveObject",
                    bcsBytes: "mockBcsBytes",
                },
            },
        });
        getObject.mockResolvedValueOnce({
            data: {
                bcs: {
                    dataType: "moveObject",
                    bcsBytes: "mockBcsBytes",
                },
            },
        });

        // Mock fromBase64 to simulate the decoding process
        (fromBase64 as any).mockReturnValueOnce("decodedBcsBytes");

        // Mock DynamicFieldStruct to return a resource without a blob_id
        (DynamicFieldStruct as any).mockImplementation(() => ({
            parse: () => ({ value: mockResource }),
        }));

        const result = await fetchResource(mockClient, "0x1", "/path", seenResources);

        // Since the resource does not have a blob_id, the function should return NOT_FOUND
        expect(result).toBe(HttpStatusCodes.NOT_FOUND);
    });

    test("should return NOT_FOUND if dynamic fields are not found", async () => {
        const seenResources = new Set<string>();

        // Mock to return no redirect
        (checkRedirect as any).mockResolvedValueOnce(null);

        // Mock to simulate that dynamic fields are not found
        getObject.mockResolvedValueOnce(undefined);

        const result = await fetchResource(mockClient, "0x1", "/path", seenResources);

        // Check that the function returns NOT_FOUND
        expect(result).toBe(HttpStatusCodes.NOT_FOUND);
    });
});
