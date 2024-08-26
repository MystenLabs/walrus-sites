// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { afterEach, describe, expect, test, vi } from 'vitest';
import { resolveSuiNsAddress, hardcodedSubdmains } from './suins';
import { SuiClient } from "@mysten/sui/client";

const resolveSuiNsAddressTestCases: [string, string | null][] = [
    ["subname", "0x123"],
    ["example", "0xabc"],
    ["nonexistent", null],
];

describe('resolveSuiNsAddress', () => {
    afterEach(() => {
        vi.clearAllMocks();
    });

    test.each(resolveSuiNsAddressTestCases)(
        'should resolve the SuiNS address for %s',
        async (input, expected) => {
            const mockClient = {
                call: vi.fn().mockResolvedValueOnce(expected)
            } as unknown as SuiClient;

            const result = await resolveSuiNsAddress(mockClient, input);
            expect(result).toBe(expected);
            expect(mockClient.call).toHaveBeenCalledWith("suix_resolveNameServiceAddress",
                [`${input}.sui`]);
        }
    );

    test('should return null for an unknown SuiNS address', async () => {
        const mockClient = {
            call: vi.fn().mockResolvedValueOnce(null)
        } as unknown as SuiClient;

        const result = await resolveSuiNsAddress(mockClient, "unknown");
        expect(result).toBeNull();
        expect(mockClient.call).toHaveBeenCalledWith("suix_resolveNameServiceAddress",
            ["unknown.sui"]);
    });
});
