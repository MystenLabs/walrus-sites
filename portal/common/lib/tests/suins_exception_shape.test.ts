// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { describe, it, expect } from "vitest";
import { SuiGrpcClient } from "@mysten/sui/grpc";
import { SuinsClient } from "@mysten/suins";
import { isNameNotRegisteredError, RPCSelector } from "@lib/rpc_selector";
import { parsePriorityUrlList } from "@lib/priority_executor";

/**
 * Integration tests — confirm that an unregistered SuiNS name is handled as
 * "not found" on real testnet.
 *
 * Requires RPC_URL_LIST and MAINNET_RPC_URL (set via .env.test). We error rather
 * than skip when either is absent, so the mainnet drift guard below can't silently
 * vanish. Hits real testnet (and mainnet for the drift guard).
 */
const testnetRpc = process.env.RPC_URL_LIST;
const mainnetRpc = process.env.MAINNET_RPC_URL;
if (!testnetRpc || !mainnetRpc) {
    throw new Error(
        "RPC_URL_LIST and MAINNET_RPC_URL must be set to run the SuiNS network tests " +
            "(normally provided by .env.test)",
    );
}

describe("SuiNS name resolution on real testnet", () => {
    const rpcSelector = new RPCSelector(parsePriorityUrlList(testnetRpc), "testnet");

    it("returns null for an unregistered name", async () => {
        // A name we are confident is not registered. Random suffix avoids
        // collisions if someone happens to register it in the future.
        const name = `walrus-portal-test-nonexistent-${Date.now()}.sui`;

        const result = await rpcSelector.getNameRecord(name);
        expect(result).toBeNull();
    }, 30_000);
});

/**
 * Drift guard for the gRPC not-found error shape, checked against mainnet — the
 * canonical, stable SuiNS registry (testnet is periodically reset).
 *
 * `getNameRecord` returns null for unregistered names only because
 * `isNameNotRegisteredError` recognises the error the SDK throws for a missing object —
 * and that detection is based on the error's message string. This test captures
 * the *raw* error from the SDK and asserts the detector still matches it. If it
 * fails, the SDK changed how it signals "not found": update `isNameNotRegisteredError`
 * in rpc_selector.ts to match the new shape.
 *
 * Uses MAINNET_RPC_URL directly (not RPC_URL_LIST, which is testnet).
 */
describe("gRPC not-found error shape (mainnet drift guard)", () => {
    it("not-found error is still detected by isNameNotRegisteredError", async () => {
        const suins = new SuinsClient({
            client: new SuiGrpcClient({ baseUrl: mainnetRpc, network: "mainnet" }),
            network: "mainnet",
        });
        const name = `walrus-portal-drift-guard-${Date.now()}.sui`;

        let thrown: unknown;
        try {
            await suins.getNameRecord(name);
        } catch (e) {
            thrown = e;
        }

        expect(
            thrown,
            "SDK no longer throws for a missing name — revisit getNameRecord not-found handling",
        ).toBeDefined();
        expect(
            isNameNotRegisteredError(thrown),
            `isNameNotRegisteredError did not detect the SDK's not-found error: ${
                (thrown as Error)?.message
            }`,
        ).toBe(true);
    }, 30_000);
});
