// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";

// Mock instrumentation to avoid port conflicts with instrumentation.test.ts.
vi.mock("@lib/instrumentation", () => ({
    instrumentationFacade: {
        recordAggregatorTime: vi.fn(),
        bumpAggregatorFailRequests: vi.fn(),
        bumpBlobUnavailableRequests: vi.fn(),
        increaseRequestsMade: vi.fn(),
    },
}));

// `aggregatorTimeoutMs` is computed at module load from
// `process.env.AGGREGATOR_REQUEST_TIMEOUT_MS`. To cover the resolve branches we
// stub the env, reset the module registry, and dynamically re-import.
async function loadAggregatorTimeoutMs(): Promise<number> {
    vi.resetModules();
    const mod = await import("@lib/url_fetcher");
    return mod.aggregatorTimeoutMs;
}

describe("AGGREGATOR_REQUEST_TIMEOUT_MS resolution", () => {
    beforeEach(() => {
        vi.unstubAllEnvs();
    });

    afterEach(() => {
        vi.unstubAllEnvs();
        vi.resetModules();
    });

    it("falls back to the 10s default when the env var is unset", async () => {
        vi.stubEnv("AGGREGATOR_REQUEST_TIMEOUT_MS", "");
        await expect(loadAggregatorTimeoutMs()).resolves.toBe(10_000);
    });

    it("uses the parsed value when a positive number is provided", async () => {
        vi.stubEnv("AGGREGATOR_REQUEST_TIMEOUT_MS", "2500");
        await expect(loadAggregatorTimeoutMs()).resolves.toBe(2500);
    });

    it.each([
        ["a non-numeric string", "not-a-number"],
        ["zero", "0"],
        ["a negative number", "-1"],
    ])("throws on %s", async (_label, raw) => {
        vi.stubEnv("AGGREGATOR_REQUEST_TIMEOUT_MS", raw);
        await expect(loadAggregatorTimeoutMs()).rejects.toThrow(
            /AGGREGATOR_REQUEST_TIMEOUT_MS must be a positive number/,
        );
    });
});
