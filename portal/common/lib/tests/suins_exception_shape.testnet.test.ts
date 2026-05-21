// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { describe, it, expect } from "vitest";
import { RPCSelector } from "@lib/rpc_selector";
import { parsePriorityUrlList } from "@lib/priority_executor";

/**
 * Integration test — confirms that RPCSelector.getNameRecord returns null
 * for an unregistered SuiNS name on real testnet, without retrying all FNs.
 *
 * Skipped unless RPC_URL_LIST is set (e.g. via .env.test). Hits real testnet
 * so it's a few seconds and may be flaky if all FNs in the priority list are
 * down at once.
 *
 * Before the fix, getNameRecord would throw an AggregateError after
 * retrying every FN. Now it returns null on the first notExists response.
 */
const rpcEnv = process.env.RPC_URL_LIST;

describe.skipIf(!rpcEnv)("SuiNS name resolution on real testnet", () => {
    const rpcSelector = new RPCSelector(parsePriorityUrlList(rpcEnv ?? ""), "testnet");

    it("returns null for an unregistered name", async () => {
        // A name we are confident is not registered. Random suffix avoids
        // collisions if someone happens to register it in the future.
        const name = `walrus-portal-test-nonexistent-${Date.now()}.sui`;

        const result = await rpcSelector.getNameRecord(name);
        expect(result).toBeNull();
    }, 30_000);
});
