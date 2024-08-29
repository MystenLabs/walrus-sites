// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { beforeEach, describe, expect, Mock, test, vi } from 'vitest';
import { resolveSuiNsAddress } from './suins';
import { SuiClient } from "@mysten/sui/client";

describe('resolveSuiNsAddress', () => {
    const mockClient = {
        call: vi.fn()
    } as unknown as SuiClient;

    beforeEach(() => {
        vi.clearAllMocks();
    });

    test('should resolve known SuiNS addresses', async () => {
        const cases = [
            ["subname", "0x123"],
            ["example", "0xabc"]
        ];

        for (const [input, expected] of cases) {
            (mockClient.call as Mock).mockResolvedValueOnce(expected);
            const result = await resolveSuiNsAddress(mockClient, input);
            expect(result).toBe(expected);
            expect(mockClient.call).toHaveBeenCalledWith("suix_resolveNameServiceAddress",
                [`${input}.sui`]);
        }
    });

    test('should return null for an unknown SuiNS address', async () => {
        (mockClient.call as Mock).mockResolvedValueOnce(null);
        const result = await resolveSuiNsAddress(mockClient, "unknown");
        expect(result).toBeNull();
        expect(mockClient.call).toHaveBeenCalledWith("suix_resolveNameServiceAddress",
            ["unknown.sui"]);
    });
});
