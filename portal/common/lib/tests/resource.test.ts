// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

// Import necessary functions and types
import { beforeEach, describe, expect, test, vi } from "vitest";
import { ResourceFetcher } from "@lib/resource";
import { RPCSelector } from "@lib/rpc_selector";
import { HttpStatusCodes } from "@lib/http/http_status_codes";
import { checkRedirect } from "@lib/redirects";
import { SuiClientTypes } from "@mysten/sui/client";
import { parsePriorityUrlList } from "@lib/priority_executor";

vi.mock("@mysten/sui/utils", () => ({
    deriveDynamicFieldID: vi.fn(() => "0xdynamicFieldId"),
}));

// Mock checkRedirect
vi.mock("../src/redirects", () => ({
    checkRedirect: vi.fn(),
}));

// Builds a fetched object with BCS content present; the parsed value is supplied
// by the mocked DynamicFieldStruct below.
// TODO(tech-debt): partial mock — cast because SuiClientTypes.Object also requires
// owner/type/previousTransaction/objectBcs/json, unused by these tests.
function mockSiteObject(
    overrides: Partial<SuiClientTypes.Object<{ content: true; display: true }>> = {},
): SuiClientTypes.Object<{ content: true; display: true }> {
    return {
        objectId: "",
        version: "1",
        digest: "",
        content: new Uint8Array([1]),
        display: null,
        ...overrides,
    } as SuiClientTypes.Object<{ content: true; display: true }>;
}

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
    const rpcPriorityUrls = parsePriorityUrlList(process.env.RPC_URL_LIST!);
    console.log("RPC URLS:", rpcPriorityUrls);
    const rpcSelector = new RPCSelector(rpcPriorityUrls, "testnet");
    const resourceFetcher = new ResourceFetcher(rpcSelector, "0x123");
    // Mock SuiClient methods
    const multiGetObjects = vi.spyOn(rpcSelector, "multiGetObjects");

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
        multiGetObjects.mockResolvedValueOnce([mockSiteObject(), mockSiteObject()]);

        const result = await resourceFetcher.fetchResource("0x1", "/path", new Set());
        expect(result).toEqual({
            blob_id: "0xresourceBlobId",
            objectId: "0xdynamicFieldId",
            version: "1",
        });
        expect(checkRedirect).toHaveBeenCalledTimes(1);
    });

    test("should follow redirect and recursively fetch resource", async () => {
        // checkRedirect is sync: a mockResolvedValueOnce would return a
        // Promise — truthy — and the redirect branch would recurse with a
        // Promise as the objectId, passing by accident. vi.mocked keeps the
        // stub typed to string | null so that can't recur.
        vi.mocked(checkRedirect).mockReturnValueOnce("0xredirectTarget").mockReturnValueOnce(null);

        multiGetObjects
            .mockResolvedValueOnce([mockSiteObject(), mockSiteObject()])
            .mockResolvedValueOnce([mockSiteObject(), mockSiteObject()]);

        const result = await resourceFetcher.fetchResource(
            "0x26dc2460093a9d6d31b58cb0ed1e72b19d140542a49be7472a6f25d542cb5cc3",
            "/path",
            new Set(),
        );

        // Verify the results
        expect(result).toEqual({
            blob_id: "0xresourceBlobId",
            objectId: "0xdynamicFieldId",
            version: "1",
        });
        expect(checkRedirect).toHaveBeenCalledTimes(2);
        // The recursion actually targets the redirect id.
        expect(multiGetObjects).toHaveBeenNthCalledWith(
            2,
            ["0xredirectTarget", "0xdynamicFieldId"],
            { content: true, display: true },
        );
    });

    test("should return NOT_FOUND if the resource does not contain a blob_id", async () => {
        const seenResources = new Set<string>();
        vi.mocked(checkRedirect).mockReturnValueOnce(null);
        multiGetObjects.mockResolvedValue([
            mockSiteObject({ version: "1.0" }),
            new Error("Object 0xdynamicFieldId not found"),
        ]);

        const result = await resourceFetcher.fetchResource("0x1", "/path", seenResources);

        // Since the resource does not have a blob_id, the function should return NOT_FOUND
        expect(result).toBe(HttpStatusCodes.NOT_FOUND);
    });

    test("serves an orphaned resource when the site object is missing (SEW-1037)", async () => {
        vi.mocked(checkRedirect).mockReturnValueOnce(null);
        multiGetObjects.mockResolvedValue([new Error("Object 0xab not found"), mockSiteObject()]);

        const result = await resourceFetcher.fetchResource("0x1", "/path", new Set());

        // Pre-existing behavior, pinned until SEW-1037 decides otherwise: the
        // orphaned dynamic field is still served.
        expect(result).toEqual({
            blob_id: "0xresourceBlobId",
            objectId: "0xdynamicFieldId",
            version: "1",
        });
        // The site miss must not be treated as a redirect candidate.
        expect(checkRedirect).not.toHaveBeenCalled();
    });

    test("should return NOT_FOUND if dynamic fields are not found", async () => {
        const seenResources = new Set<string>();

        // Mock to return no redirect
        vi.mocked(checkRedirect).mockReturnValueOnce(null);

        // Mock to simulate that dynamic fields are not found
        multiGetObjects.mockResolvedValue([
            mockSiteObject({ objectId: "mockObjectId", digest: "mockDigest", version: "1.0" }),
            new Error("Object 0xdynamicFieldId not found"),
        ]);

        const result = await resourceFetcher.fetchResource("0x1", "/path", seenResources);

        // Check that the function returns NOT_FOUND
        expect(result).toBe(HttpStatusCodes.NOT_FOUND);
    });
});
