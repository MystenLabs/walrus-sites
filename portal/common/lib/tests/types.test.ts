// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { optionalRangeToHeaders, Range } from "@lib/types/index";
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
        [2, 1]
    ])('throw error when start = %s and end = %s "', (start, end) => {
        expect(() => optionalRangeToHeaders({ start, end } as Range)).toThrowError(
            `Invalid range: start=${start} end=${end}`,
        );
    });
});
