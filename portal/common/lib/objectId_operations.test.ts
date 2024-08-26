// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { describe, expect, test } from 'vitest';
import { subdomainToObjectId, HEXtoBase36, Base36ToHEX } from './objectId_operations';

// Test cases for subdomainToObjectId
const subdomainToObjectIdTestCases: [string, string | null][] = [
    ["29gjzk8yjl1v7zm2etee1siyzaqfj9jaru5ufs6yyh1yqsgun2",
        // Example Base36 subdomain
        "0x5ac988828a0c9842d91e6d5bdd9552ec9fcdddf11c56bf82dff6d5566685a31e"],
    ["invalidsubdomain", null], // Invalid subdomain that doesn't map to a valid object ID
];

describe('subdomainToObjectId', () => {
    subdomainToObjectIdTestCases.forEach(([input, expected]) => {
        test(`Converting subdomain ${input} should return ${expected ? expected : 'null'}`, () => {
            const result = subdomainToObjectId(input);
            expect(result).toEqual(expected);
        });
    });
});

// Test cases for HEXtoBase36 and Base36ToHEX
const HEXtoBase36TestCases: [string, string][] = [
    ["0x5ac988828a0c9842d91e6d5bdd9552ec9fcdddf11c56bf82dff6d5566685a31e",
        "29gjzk8yjl1v7zm2etee1siyzaqfj9jaru5ufs6yyh1yqsgun2"], // Valid HEX to Base36
    ["0x0000000000000000000000000000000000000001", "1"], // Minimal HEX to Base36
];

describe('HEXtoBase36 and Base36ToHEX', () => {
    HEXtoBase36TestCases.forEach(([hexInput, base36Expected]) => {
        test(`Converting HEX ${hexInput} to Base36 should return ${base36Expected}`, () => {
            const result = HEXtoBase36(hexInput);
            expect(result).toBe(base36Expected);
        });

        test(`Converting Base36 ${base36Expected} back to HEX should return ${hexInput}`, () => {
            const result = Base36ToHEX(base36Expected);
            expect(result).toBe(hexInput);
        });
    });
});
