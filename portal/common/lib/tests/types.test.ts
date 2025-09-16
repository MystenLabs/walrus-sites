// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import {
    isResource,
    isVersionedResource,
    optionalRangeToHeaders,
    Range,
    isRoutes,
} from "@lib/types/index";
import { describe, it, expect } from "vitest";

describe("happy paths of optionalRangeToHeaders should", () => {
    it.each([
        [{ start: 0, end: 10 }, "bytes=0-10"],
        [{ start: null, end: 10 }, "bytes=-10"],
        [{ start: 10, end: null }, "bytes=10-"],
        [null, undefined],
    ])('convert range %o into "%s"', (range, expected) => {
        expect(optionalRangeToHeaders(range as Range).range).toBe(expected);
    });
});

describe("cases where optionalRangeToHeaders should", () => {
    it.each([
        [null, null],
        [null, -1],
        [-1, null],
        [2, 1],
    ])('throw error when start = %s and end = %s "', (start, end) => {
        expect(() => optionalRangeToHeaders({ start, end } as Range)).toThrowError(
            `Invalid range: start=${start} end=${end}`,
        );
    });
});

describe("isResource", () => {
    it("should return true when it's definitely a resource", () => {
        expect(
            isResource({
                path: "index.html",
                headers: { "cache-control": "public, max-age=86400" },
                blob_id: "9Jws-C9zE99FwWex6dsVbpaaaaaIk7Ko7No-8mKfgRk",
                blob_hash: "O1NaFqZZkVOp+2hJfbf6s+a02JHMiJh7q0DZ+Dyp8og",
                range: null,
            }),
        ).toBeTruthy();
    });
    it("should return false when path is a number", () => {
        expect(
            isResource({
                path: 123,
                headers: { "cache-control": "public, max-age=86400" },
                blob_id: "9Jws-C9zE99FwWex6dsVbpaaaaaIk7Ko7No-8mKfgRk",
                blob_hash: "O1NaFqZZkVOp+2hJfbf6s+a02JHMiJh7q0DZ+Dyp8og",
                range: null,
            }),
        ).toBeFalsy();
    });
    it("should return false when blob_id is a number", () => {
        expect(
            isResource({
                path: "index.html",
                headers: { "cache-control": "public, max-age=86400" },
                blob_id: 123,
                blob_hash: "O1NaFqZZkVOp+2hJfbf6s+a02JHMiJh7q0DZ+Dyp8og",
                range: null,
            }),
        ).toBeFalsy();
    });
    it("should return false when headers is not an object", () => {
        expect(
            isResource({
                path: "index.html",
                headers: "not an object",
                blob_id: "9Jws-C9zE99FwWex6dsVbpaaaaaIk7Ko7No-8mKfgRk",
                blob_hash: "O1NaFqZZkVOp+2hJfbf6s+a02JHMiJh7q0DZ+Dyp8og",
                range: null,
            }),
        ).toBeFalsy();
    });
    it("should return false when blob_hash is not a string", () => {
        expect(
            isResource({
                path: "index.html",
                headers: { "cache-control": "public, max-age=86400" },
                blob_id: "9Jws-C9zE99FwWex6dsVbpaaaaaIk7Ko7No-8mKfgRk",
                blob_hash: 123,
                range: null,
            }),
        ).toBeFalsy();
    });
});

describe("isVersionedResource", () => {
    it("should return true when it's definitely a resource AND versioned!", () => {
        expect(
            isVersionedResource({
                path: "index.html",
                headers: { "cache-control": "public, max-age=86400" },
                blob_id: "9Jws-C9zE99FwWex6dsVbpaaaaaIk7Ko7No-8mKfgRk",
                blob_hash: "O1NaFqZZkVOp+2hJfbf6s+a02JHMiJh7q0DZ+Dyp8og",
                range: null,
                version: 1,
                objectId: "0xf7060e42698c5124afba30354abc845c97d3a841df887b2b6b68f4b689f863bd",
            }),
        ).toBeTruthy();
    });
    it("should return false when version is missing!", () => {
        expect(
            isVersionedResource({
                path: "index.html",
                headers: { "cache-control": "public, max-age=86400" },
                blob_id: "9Jws-C9zE99FwWex6dsVbpaaaaaIk7Ko7No-8mKfgRk",
                blob_hash: "O1NaFqZZkVOp+2hJfbf6s+a02JHMiJh7q0DZ+Dyp8og",
                range: null,
                objectId: "0xf7060e42698c5124afba30354abc845c97d3a841df887b2b6b68f4b689f863bd",
            }),
        ).toBeFalsy();
    });
    it("should return false when objectId is missing!", () => {
        expect(
            isVersionedResource({
                path: "index.html",
                headers: { "cache-control": "public, max-age=86400" },
                blob_id: "9Jws-C9zE99FwWex6dsVbpaaaaaIk7Ko7No-8mKfgRk",
                blob_hash: "O1NaFqZZkVOp+2hJfbf6s+a02JHMiJh7q0DZ+Dyp8og",
                range: null,
                version: 1,
            }),
        ).toBeFalsy();
    });
});

describe("isRoutes", () => {
    it("should return true when it's definitely a routes object", () => {
        expect(
            isRoutes({
                routes_list: new Map([
                    ["/index.html", { path: "/index.html" }],
                    ["/about", { path: "/about" }],
                ]),
            }),
        ).toBeTruthy();
    });

    it.each([
        [{ other_property: "value" }, "routes_list is missing"],
        [{ routes_list: {} }, "routes_list is not a Map but an object"],
        [{ routes_list: [] }, "routes_list is not a Map but an array"],
        [{ routes_list: "not a map" }, "routes_list is not a Map but a string"],
        [null, "obj is null"],
        [undefined, "obj is undefined"],
    ])("should return false when %s", (input, description) => {
        expect(isRoutes(input)).toBeFalsy();
    });
});
