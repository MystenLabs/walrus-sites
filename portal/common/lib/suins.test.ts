// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { beforeEach, describe, expect, test, vi } from 'vitest';
import { SuiNSResolver } from './suins';
import { RPCSelector } from './rpc_selector';
import { NameRecord } from './types';

describe('resolveSuiNsAddress', () => {
    const rpcSelector = new RPCSelector(process.env.RPC_URL_LIST!.split(','), "testnet")
    const suiNSResolver = new SuiNSResolver(
        rpcSelector
    );

    beforeEach(() => {
        vi.clearAllMocks();
    });

    test('should resolve known SuiNS addresses', async () => {
        const cases = [
            // The most common case.
            ["subname", {
                name: "dummyName",
                nftId: "dummyNftId",
                targetAddress: "dummyTargetAddress",
                expirationTimestampMs: 1234567890,
                data: { key: "dummyValue" },
                avatar: "dummyAvatar",
                contentHash: "dummyContentHash",
                walrusSiteId: "0x57414C525553"
            }],
            // Deprecated case where the walrusSiteId is not set, but the targetAddress is used
            //  instead:
            ["docs", {
              name: 'docs.sui',
              nftId: '0xcc8ee266bc1f07b3218696912c7e5244aeb0e32c8c9befec059e3527558e4063',
              targetAddress: '0x57414C525553',
              expirationTimestampMs: '1749826519160',
              data: {},
              avatar: undefined,
              contentHash: undefined,
              walrusSiteId: undefined
            }]
        ];

        for (const [input, expected] of cases) {
            vi.spyOn(rpcSelector, 'getNameRecord').mockResolvedValueOnce(expected as NameRecord);
            const result: string = await suiNSResolver.resolveSuiNsAddress(input as string);
            expect(result).toBe("0x57414C525553");
            expect(rpcSelector.getNameRecord).toHaveBeenCalledWith(`${input}.sui`);
        }
    });

    test('should return null for an unknown SuiNS address', async () => {
        vi.spyOn(rpcSelector, 'getNameRecord').mockResolvedValueOnce(null);
        const result = await suiNSResolver.resolveSuiNsAddress("unknown");
        expect(rpcSelector.getNameRecord).toHaveBeenCalledWith(
            "unknown.sui"
        );
        expect(result).toBeNull();
    });
});
