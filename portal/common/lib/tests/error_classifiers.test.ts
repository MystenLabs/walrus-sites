// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { describe, it, expect } from "vitest";
import { isObjectNotFoundError, isNameNotRegisteredError } from "@lib/rpc_selector";

/**
 * False-positive pins for the error classifiers.
 *
 * The live-network drift guards (grpc_get_objects_shape.test.ts,
 * suins_exception_shape.test.ts) pin the false-negative side: a real miss keeps
 * being detected. These unit cases pin the opposite side — a transient error
 * misread as an authoritative miss would 404 a live site instead of failing
 * over to the next RPC, and the `code`-is-string shield plus the strict
 * message regex are the only defenses.
 */
describe("isObjectNotFoundError — false-positive pins", () => {
    it("accepts a genuine miss (baseline)", () => {
        expect(isObjectNotFoundError(new Error("Object 0xdeadbeef not found"))).toBe(true);
    });

    it("rejects a miss-shaped message that carries a gRPC status code", () => {
        const error = new Error("Object 0xab not found");
        (error as Error & { code: string }).code = "NOT_FOUND";
        expect(isObjectNotFoundError(error)).toBe(false);
        expect(isNameNotRegisteredError(error)).toBe(false);
    });

    it("rejects a plain HTTP-style 'Not Found' with no object id", () => {
        expect(isObjectNotFoundError(new Error("Not Found"))).toBe(false);
    });

    it("rejects a non-hex object id (pins regex strictness)", () => {
        expect(isObjectNotFoundError(new Error("Object 0xzz not found"))).toBe(false);
    });

    it("rejects non-Error inputs", () => {
        expect(isObjectNotFoundError("Object 0x1 not found")).toBe(false);
        expect(isObjectNotFoundError(null)).toBe(false);
        expect(isObjectNotFoundError(undefined)).toBe(false);
    });

    it("rejects a wrapper error that merely contains the miss text (pins the ^ anchor)", () => {
        const aggregate = new AggregateError([], "fetch failed: Object 0x1 not found");
        expect(isObjectNotFoundError(aggregate)).toBe(false);
    });

    it("accepts uppercase miss text (pins the `i` flag as intended)", () => {
        expect(isObjectNotFoundError(new Error("OBJECT 0X6 NOT FOUND"))).toBe(true);
    });
});

describe("isNameNotRegisteredError — false-positive pins", () => {
    it("accepts a genuine not-registered error (baseline)", () => {
        expect(isNameNotRegisteredError(new Error("name.sui is not registered"))).toBe(true);
    });

    it("rejects the same message when it carries a gRPC status code", () => {
        const error = new Error("name.sui is not registered");
        (error as Error & { code: string }).code = "UNAVAILABLE";
        expect(isNameNotRegisteredError(error)).toBe(false);
    });

    it("rejects non-Error inputs", () => {
        expect(isNameNotRegisteredError("name.sui is not registered")).toBe(false);
        expect(isNameNotRegisteredError(null)).toBe(false);
    });
});
