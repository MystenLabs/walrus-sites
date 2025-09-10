// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { describe, expect, test } from 'vitest';
import { base64UrlSafeEncode } from '@lib/url_safe_base64';

// Test cases for base64UrlSafeEncode
const base64UrlSafeEncodeTestCases: [Uint8Array, string][] = [
    [new Uint8Array([104, 101, 108, 108, 111]), 'aGVsbG8'], // "hello"
    [new Uint8Array([119, 111, 114, 108, 100]), 'd29ybGQ'], // "world"
    [new Uint8Array([]), ''], // empty array
];

describe('base64UrlSafeEncode', () => {
    base64UrlSafeEncodeTestCases.forEach(([input, expected]) => {
        test(`Encoding ${input.toString()} should return ${expected}`, () => {
            const result = base64UrlSafeEncode(input);
            expect(result).toBe(expected);
        });
    });
});
