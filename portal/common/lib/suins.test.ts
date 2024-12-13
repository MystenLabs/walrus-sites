// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { beforeEach, describe, expect, test, vi } from 'vitest';
import { resolveSuiNsAddress } from './suins';
import rpcSelector from './rpc_selector';

describe('resolveSuiNsAddress', () => {
    beforeEach(() => {
        vi.clearAllMocks();
    });

    test('should resolve known SuiNS addresses', async () => {
        const cases = [
            ["subname", "0x123"],
            ["example", "0xabc"]
        ];

        for (const [input, expected] of cases) {
            // Mock the rpcSelectorSingleton.call method
            vi.spyOn(rpcSelector, 'call').mockResolvedValueOnce(expected);

            const result = await resolveSuiNsAddress(input);

            expect(result).toBe(expected);
            expect(rpcSelector.call).toHaveBeenCalledWith(
                "call",
                ["suix_resolveNameServiceAddress", [`${input}.sui`]]
            );
        }
    });

    test('should return null for an unknown SuiNS address', async () => {
        // Mock the rpcSelectorSingleton.call method to return null
        vi.spyOn(rpcSelector, 'call').mockResolvedValueOnce(null);

        const result = await resolveSuiNsAddress("unknown");

        expect(result).toBeNull();
        expect(rpcSelector.call).toHaveBeenCalledWith(
            "call",
            ["suix_resolveNameServiceAddress", ["unknown.sui"]]
        );
    });
});
