// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { describe, it, expect, vi, beforeEach } from "vitest";
import { RPCSelector } from "@lib/rpc_selector";
import { PriorityExecutor, PriorityUrl } from "@lib/priority_executor";

/**
 * Tests that RPCSelector.getNameRecord handles ObjectError(notExists)
 * correctly: returns null (no retries) instead of propagating the error.
 *
 * The @mysten/sui SDK throws ObjectError(code: "notExists") when the FN
 * cleanly responds that a dynamic field doesn't exist — which is exactly
 * what happens for any unregistered SuiNS name. This is an authoritative
 * answer (name registration is determinate on-chain state), so the
 * RPCSelector should stop immediately and return null.
 */

/** Mirrors @mysten/sui ObjectError shape (not publicly exported). */
class ObjectErrorLike extends Error {
    public code: string;
    constructor(code: string, message: string) {
        super(message);
        this.code = code;
        this.name = "ObjectError";
    }
}

// We construct the RPCSelector via its public constructor, but need to
// control what the underlying SuiNS client returns. We do this by
// intercepting the SuinsClient.getNameRecord call at the network level.
// Instead, we'll test at the PriorityExecutor integration level by
// creating an RPCSelector with a real executor but mocked clients.

describe("RPCSelector.getNameRecord — notExists handling", () => {
    const urls: PriorityUrl[] = [
        { url: "http://fn1.test", retries: 2, metric: 100 },
        { url: "http://fn2.test", retries: 2, metric: 200 },
    ];

    it("returns null without retrying when FN responds notExists", async () => {
        const notExistsError = new ObjectErrorLike(
            "notExists",
            "Object 0xdeadbeef does not exist",
        );

        // Track how many times the executor's callback is invoked to verify
        // no retries happen.
        let callCount = 0;

        const executor = new PriorityExecutor(urls, 0);
        const invokeSpy = vi.spyOn(executor, "invoke").mockImplementation(async (execute) => {
            // Simulate what invokeWithFailover does: call execute for the
            // first URL, which detects notExists and returns "stop".
            callCount++;
            const result = await execute(urls[0].url);
            if (result.status === "stop") {
                const wrapped = new Error(`stop from ${urls[0].url}`, { cause: result.error });
                throw new AggregateError([wrapped], "Stopped");
            }
            return result.value;
        });

        // Build RPCSelector and replace its executor with our spy.
        const rpcSelector = new RPCSelector(urls, "testnet");
        (rpcSelector as unknown as { executor: PriorityExecutor }).executor = executor;

        // The getNameRecord catch block should detect notExists and return null.
        // But we need invokeWithFailover to actually run... Let's take a
        // different approach: test the full flow by mocking fetch.
        invokeSpy.mockRestore();

        // More direct approach: mock the global fetch to simulate the FN
        // throwing notExists. The SuinsClient uses the Sui JSON-RPC client
        // which ultimately calls fetch.
        const originalFetch = globalThis.fetch;
        let fetchCallCount = 0;
        globalThis.fetch = vi.fn().mockImplementation(async () => {
            fetchCallCount++;
            throw notExistsError;
        });

        try {
            const result = await rpcSelector.getNameRecord("nonexistent.sui");
            expect(result).toBeNull();
            // Should NOT have retried — notExists triggers "stop" on first call.
            expect(fetchCallCount).toBe(1);
        } finally {
            globalThis.fetch = originalFetch;
        }
    });

    it("retries on transient network errors", async () => {
        const rpcSelector = new RPCSelector(urls, "testnet");

        const originalFetch = globalThis.fetch;
        let fetchCallCount = 0;
        globalThis.fetch = vi.fn().mockImplementation(async () => {
            fetchCallCount++;
            throw new TypeError("fetch failed");
        });

        try {
            await expect(rpcSelector.getNameRecord("anything.sui")).rejects.toThrow();
            // Should have retried: 2 URLs × (1 initial + 2 retries) = 6 calls
            expect(fetchCallCount).toBe(6);
        } finally {
            globalThis.fetch = originalFetch;
        }
    });

    it("does not retry when notExists is mixed with prior transient failures", async () => {
        const rpcSelector = new RPCSelector(urls, "testnet");

        const originalFetch = globalThis.fetch;
        let fetchCallCount = 0;
        globalThis.fetch = vi.fn().mockImplementation(async () => {
            fetchCallCount++;
            // First call: transient error (will trigger retry-same)
            if (fetchCallCount === 1) {
                throw new TypeError("fetch failed");
            }
            // Second call: notExists (should trigger stop)
            throw new ObjectErrorLike("notExists", "Object 0xabc does not exist");
        });

        try {
            const result = await rpcSelector.getNameRecord("test.sui");
            expect(result).toBeNull();
            // 1 transient failure + 1 notExists = 2 calls total (stopped on second)
            expect(fetchCallCount).toBe(2);
        } finally {
            globalThis.fetch = originalFetch;
        }
    });
});
