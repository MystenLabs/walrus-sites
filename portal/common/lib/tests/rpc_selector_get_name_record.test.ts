// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { describe, it, expect, vi } from "vitest";
import { RPCSelector } from "@lib/rpc_selector";
import { PriorityUrl } from "@lib/priority_executor";

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

describe("RPCSelector.getNameRecord — notExists handling", () => {
    const urls: PriorityUrl[] = [
        { url: "http://fn1.test", retries: 2, metric: 100 },
        { url: "http://fn2.test", retries: 2, metric: 200 },
    ];

    it("returns null without retrying when FN responds notExists", async () => {
        const notExistsError = new ObjectErrorLike("notExists", "Object 0xdeadbeef does not exist");

        const rpcSelector = new RPCSelector(urls, "testnet");

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
