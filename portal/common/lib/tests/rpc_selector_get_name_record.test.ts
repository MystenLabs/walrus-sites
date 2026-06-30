// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { describe, it, expect, vi, afterEach } from "vitest";
import { SuinsClient } from "@mysten/suins";
import { RPCSelector } from "@lib/rpc_selector";
import { PriorityUrl } from "@lib/priority_executor";

/**
 * Tests that RPCSelector.getNameRecord handles a "not found" response
 * correctly: returns null (no retries) instead of propagating the error.
 *
 * The gRPC core API throws a plain Error like "Object 0x… not found" when the
 * fullnode cleanly responds that a dynamic field doesn't exist — which is
 * exactly what happens for any unregistered SuiNS name (verified against real
 * testnet in suins_exception_shape.testnet.test.ts). This is an authoritative
 * answer (name registration is determinate on-chain state), so the RPCSelector
 * should stop immediately and return null.
 *
 * We stub `SuinsClient.getNameRecord` (the SDK boundary RPCSelector calls per
 * client) rather than `fetch`, so the test is independent of the gRPC-web wire
 * format and exercises exactly the failover stop/retry decision.
 */
describe("RPCSelector.getNameRecord — not-found handling", () => {
    const urls: PriorityUrl[] = [
        { url: "http://fn1.test", retries: 2, metric: 100 },
        { url: "http://fn2.test", retries: 2, metric: 200 },
    ];

    const notFoundError = () => new Error("Object 0xdeadbeef not found");
    const transientError = () => new TypeError("fetch failed");

    afterEach(() => {
        vi.restoreAllMocks();
    });

    it("returns null without retrying when FN responds not-found", async () => {
        const rpcSelector = new RPCSelector(urls, "testnet");
        const spy = vi
            .spyOn(SuinsClient.prototype, "getNameRecord")
            .mockRejectedValue(notFoundError());

        const result = await rpcSelector.getNameRecord("nonexistent.sui");
        expect(result).toBeNull();
        // Should NOT have retried — not-found triggers "stop" on first call.
        expect(spy).toHaveBeenCalledTimes(1);
    });

    it("retries on transient network errors", async () => {
        const rpcSelector = new RPCSelector(urls, "testnet");
        const spy = vi
            .spyOn(SuinsClient.prototype, "getNameRecord")
            .mockRejectedValue(transientError());

        await expect(rpcSelector.getNameRecord("anything.sui")).rejects.toThrow();
        // Should have retried: 2 URLs × (1 initial + 2 retries) = 6 calls
        expect(spy).toHaveBeenCalledTimes(6);
    });

    it("does not retry when not-found is mixed with prior transient failures", async () => {
        const rpcSelector = new RPCSelector(urls, "testnet");
        let callCount = 0;
        const spy = vi
            .spyOn(SuinsClient.prototype, "getNameRecord")
            .mockImplementation(async () => {
                callCount++;
                // First call: transient error (will trigger retry-same).
                if (callCount === 1) throw transientError();
                // Second call: not-found (should trigger stop).
                throw notFoundError();
            });

        const result = await rpcSelector.getNameRecord("test.sui");
        expect(result).toBeNull();
        // 1 transient failure + 1 not-found = 2 calls total (stopped on second).
        expect(spy).toHaveBeenCalledTimes(2);
    });

    it("retries a transport error whose message contains 'not found' (HTTP 404)", async () => {
        const rpcSelector = new RPCSelector(urls, "testnet");
        // A gRPC-web HTTP 404 surfaces as RpcError("Not Found", "NOT_FOUND"):
        // the message contains "not found", but it is a transient transport
        // failure, NOT an authoritative "name not registered" — so it must be
        // retried/failed over, never short-circuited to null.
        const transportNotFound = () =>
            Object.assign(new Error("Not Found"), { code: "NOT_FOUND" });
        const spy = vi
            .spyOn(SuinsClient.prototype, "getNameRecord")
            .mockRejectedValue(transportNotFound());

        await expect(rpcSelector.getNameRecord("docs.sui")).rejects.toThrow();
        // 2 URLs × (1 initial + 2 retries) = 6 calls.
        expect(spy).toHaveBeenCalledTimes(6);
    });
});
