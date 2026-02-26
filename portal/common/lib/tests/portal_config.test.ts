// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { describe, it, expect } from "vitest";
import { parsePortalConfigYaml } from "@lib/portal_config";

const VALID_SITE_PACKAGE = "0x26eb7ee8688da02c5f671679524e379f0b837a12f1d1d799f255b7eea260ad27";

function makeValidYaml(overrides: Record<string, unknown> = {}): string {
    const base: Record<string, unknown> = {
        network: "mainnet",
        site_package: VALID_SITE_PACKAGE,
        landing_page_oid_b36: "46f3881sp4r55fc6pcao9t93bieeejl4vr4k2uv8u4wwyx1a93",
        rpc_urls: [{ url: "https://rpc.example.com", retries: 2, metric: 100 }],
        aggregator_urls: [{ url: "https://aggregator.example.com", retries: 3, metric: 100 }],
        b36_domain_resolution: true,
        enable_blocklist: false,
        enable_allowlist: false,
        ...overrides,
    };
    // Simple YAML serialization for test purposes
    return toYaml(base);
}

function toYaml(obj: Record<string, unknown>, indent = 0): string {
    const pad = " ".repeat(indent);
    const lines: string[] = [];
    for (const [key, value] of Object.entries(obj)) {
        if (Array.isArray(value)) {
            if (value.length === 0) {
                lines.push(`${pad}${key}: []`);
                continue;
            }
            lines.push(`${pad}${key}:`);
            for (const item of value) {
                if (typeof item === "object" && item !== null) {
                    const entries = Object.entries(item as Record<string, unknown>);
                    lines.push(`${pad}  - ${entries[0][0]}: ${formatValue(entries[0][1])}`);
                    for (let i = 1; i < entries.length; i++) {
                        lines.push(`${pad}    ${entries[i][0]}: ${formatValue(entries[i][1])}`);
                    }
                } else {
                    lines.push(`${pad}  - ${formatValue(item)}`);
                }
            }
        } else {
            lines.push(`${pad}${key}: ${formatValue(value)}`);
        }
    }
    return lines.join("\n") + "\n";
}

function formatValue(value: unknown): string {
    if (typeof value === "string") return `"${value}"`;
    if (typeof value === "boolean" || typeof value === "number") return String(value);
    return String(value);
}

describe("parsePortalConfigYaml", () => {
    describe("valid configs", () => {
        it("parses a complete valid config with all fields", () => {
            const yaml = makeValidYaml({
                premium_rpc_urls: [{ url: "https://premium.example.com", retries: 1, metric: 50 }],
                domain_name_length: 21,
                bring_your_own_domain: false,
            });
            const config = parsePortalConfigYaml(yaml);

            expect(config.network).toBe("mainnet");
            expect(config.site_package).toBe(VALID_SITE_PACKAGE);
            expect(config.landing_page_oid_b36).toBe(
                "46f3881sp4r55fc6pcao9t93bieeejl4vr4k2uv8u4wwyx1a93",
            );
            expect(config.rpc_urls).toEqual([
                { url: "https://rpc.example.com", retries: 2, metric: 100 },
            ]);
            expect(config.premium_rpc_urls).toEqual([
                { url: "https://premium.example.com", retries: 1, metric: 50 },
            ]);
            expect(config.aggregator_urls).toEqual([
                { url: "https://aggregator.example.com", retries: 3, metric: 100 },
            ]);
            expect(config.domain_name_length).toBe(21);
            expect(config.b36_domain_resolution).toBe(true);
            expect(config.bring_your_own_domain).toBe(false);
            expect(config.enable_blocklist).toBe(false);
            expect(config.enable_allowlist).toBe(false);
        });

        it("parses a minimal valid config (no optional fields)", () => {
            const yaml = makeValidYaml();
            const config = parsePortalConfigYaml(yaml);

            expect(config.network).toBe("mainnet");
            expect(config.premium_rpc_urls).toBeUndefined();
            expect(config.domain_name_length).toBeUndefined();
            expect(config.bring_your_own_domain).toBeUndefined();
        });

        it("accepts testnet as network", () => {
            const yaml = makeValidYaml({ network: "testnet" });
            const config = parsePortalConfigYaml(yaml);
            expect(config.network).toBe("testnet");
        });

        it("passes metric values through correctly to PriorityUrl", () => {
            const yaml = makeValidYaml({
                rpc_urls: [
                    { url: "https://a.com", retries: 1, metric: 50 },
                    { url: "https://b.com", retries: 0, metric: 200 },
                ],
            });
            const config = parsePortalConfigYaml(yaml);

            expect(config.rpc_urls[0]).toEqual({
                url: "https://a.com",
                retries: 1,
                metric: 50,
            });
            expect(config.rpc_urls[1]).toEqual({
                url: "https://b.com",
                retries: 0,
                metric: 200,
            });
        });

        it("accepts zero retries", () => {
            const yaml = makeValidYaml({
                rpc_urls: [{ url: "https://a.com", retries: 0, metric: 100 }],
            });
            const config = parsePortalConfigYaml(yaml);
            expect(config.rpc_urls[0].retries).toBe(0);
        });

        it("accepts multiple URLs in arrays", () => {
            const yaml = makeValidYaml({
                rpc_urls: [
                    { url: "https://a.com", retries: 2, metric: 100 },
                    { url: "https://b.com", retries: 1, metric: 200 },
                    { url: "https://c.com", retries: 0, metric: 300 },
                ],
            });
            const config = parsePortalConfigYaml(yaml);
            expect(config.rpc_urls).toHaveLength(3);
        });
    });

    describe("invalid network", () => {
        it("rejects unknown network values", () => {
            const yaml = makeValidYaml({ network: "devnet" });
            expect(() => parsePortalConfigYaml(yaml)).toThrow(
                /network.*must be "testnet" or "mainnet"/,
            );
        });

        it("rejects non-string network", () => {
            const yaml = makeValidYaml({ network: 123 });
            expect(() => parsePortalConfigYaml(yaml)).toThrow(/network.*must be a string/);
        });
    });

    describe("invalid site_package", () => {
        it("rejects site_package without 0x prefix", () => {
            const yaml = makeValidYaml({
                site_package: "26eb7ee8688da02c5f671679524e379f0b837a12f1d1d799f255b7eea260ad27",
            });
            expect(() => parsePortalConfigYaml(yaml)).toThrow(/site_package.*0x \+ 64 hex/);
        });

        it("rejects site_package with wrong length", () => {
            const yaml = makeValidYaml({ site_package: "0xabcd" });
            expect(() => parsePortalConfigYaml(yaml)).toThrow(/site_package.*0x \+ 64 hex/);
        });

        it("rejects site_package with invalid hex chars", () => {
            const yaml = makeValidYaml({
                site_package: "0xZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZ",
            });
            expect(() => parsePortalConfigYaml(yaml)).toThrow(/site_package.*0x \+ 64 hex/);
        });
    });

    describe("invalid URLs", () => {
        it("rejects invalid URL in rpc_urls", () => {
            const yaml = makeValidYaml({
                rpc_urls: [{ url: "not-a-url", retries: 2, metric: 100 }],
            });
            expect(() => parsePortalConfigYaml(yaml)).toThrow(/Invalid URL.*rpc_urls/);
        });

        it("rejects invalid URL in aggregator_urls", () => {
            const yaml = makeValidYaml({
                aggregator_urls: [{ url: "not-valid", retries: 2, metric: 100 }],
            });
            expect(() => parsePortalConfigYaml(yaml)).toThrow(/Invalid URL.*aggregator_urls/);
        });

        it("rejects invalid URL in premium_rpc_urls", () => {
            const yaml = makeValidYaml({
                premium_rpc_urls: [{ url: "://missing-scheme", retries: 1, metric: 100 }],
            });
            expect(() => parsePortalConfigYaml(yaml)).toThrow(/Invalid URL.*premium_rpc_urls/);
        });
    });

    describe("invalid retries", () => {
        it("rejects negative retries", () => {
            const yaml = makeValidYaml({
                rpc_urls: [{ url: "https://a.com", retries: -1, metric: 100 }],
            });
            expect(() => parsePortalConfigYaml(yaml)).toThrow(/retries.*non-negative integer/);
        });

        it("rejects non-integer retries", () => {
            const yaml = makeValidYaml({
                rpc_urls: [{ url: "https://a.com", retries: 1.5, metric: 100 }],
            });
            expect(() => parsePortalConfigYaml(yaml)).toThrow(/retries.*non-negative integer/);
        });
    });

    describe("invalid metric", () => {
        it("rejects non-integer metric", () => {
            const yaml = makeValidYaml({
                rpc_urls: [{ url: "https://a.com", retries: 1, metric: 1.5 }],
            });
            expect(() => parsePortalConfigYaml(yaml)).toThrow(/metric.*must be an integer/);
        });
    });

    describe("empty required arrays", () => {
        it("rejects empty rpc_urls", () => {
            const yaml = makeValidYaml({ rpc_urls: [] });
            expect(() => parsePortalConfigYaml(yaml)).toThrow(/rpc_urls.*must not be empty/);
        });

        it("rejects empty aggregator_urls", () => {
            const yaml = makeValidYaml({ aggregator_urls: [] });
            expect(() => parsePortalConfigYaml(yaml)).toThrow(/aggregator_urls.*must not be empty/);
        });
    });

    describe("missing required fields", () => {
        it("rejects missing rpc_urls", () => {
            // Build YAML without rpc_urls
            const obj: Record<string, unknown> = {
                network: "mainnet",
                site_package: VALID_SITE_PACKAGE,
                landing_page_oid_b36: "abc123",
                aggregator_urls: [{ url: "https://a.com", retries: 1, metric: 100 }],
                b36_domain_resolution: true,
                enable_blocklist: false,
                enable_allowlist: false,
            };
            expect(() => parsePortalConfigYaml(toYaml(obj))).toThrow(/rpc_urls.*must be an array/);
        });

        it("rejects missing network", () => {
            const obj: Record<string, unknown> = {
                site_package: VALID_SITE_PACKAGE,
                landing_page_oid_b36: "abc123",
                rpc_urls: [{ url: "https://a.com", retries: 1, metric: 100 }],
                aggregator_urls: [{ url: "https://a.com", retries: 1, metric: 100 }],
                b36_domain_resolution: true,
                enable_blocklist: false,
                enable_allowlist: false,
            };
            expect(() => parsePortalConfigYaml(toYaml(obj))).toThrow(/network.*must be a string/);
        });
    });

    describe("invalid optional fields", () => {
        it("rejects non-positive domain_name_length", () => {
            const yaml = makeValidYaml({ domain_name_length: 0 });
            expect(() => parsePortalConfigYaml(yaml)).toThrow(
                /domain_name_length.*positive integer/,
            );
        });

        it("rejects negative domain_name_length", () => {
            const yaml = makeValidYaml({ domain_name_length: -5 });
            expect(() => parsePortalConfigYaml(yaml)).toThrow(
                /domain_name_length.*positive integer/,
            );
        });

        it("rejects non-boolean bring_your_own_domain", () => {
            const yaml = makeValidYaml({ bring_your_own_domain: "yes" });
            expect(() => parsePortalConfigYaml(yaml)).toThrow(
                /bring_your_own_domain.*must be a boolean/,
            );
        });
    });

    describe("malformed YAML", () => {
        it("rejects completely invalid YAML", () => {
            expect(() => parsePortalConfigYaml("{{{{invalid")).toThrow(/Failed to parse YAML/);
        });

        it("rejects YAML that is not an object", () => {
            expect(() => parsePortalConfigYaml("just a string")).toThrow(
                /must be an object at the top level/,
            );
        });

        it("rejects YAML array at top level", () => {
            // An array is technically an object in JS, so validation falls through
            // to the first required field check
            expect(() => parsePortalConfigYaml("- item1\n- item2")).toThrow();
        });
    });

    describe("boolean validation", () => {
        it("rejects string 'true' for boolean fields", () => {
            const yaml = makeValidYaml({ enable_blocklist: "true" });
            expect(() => parsePortalConfigYaml(yaml)).toThrow(
                /enable_blocklist.*must be a boolean/,
            );
        });

        it("rejects numeric 1 for boolean fields", () => {
            const yaml = makeValidYaml({ b36_domain_resolution: 1 });
            expect(() => parsePortalConfigYaml(yaml)).toThrow(
                /b36_domain_resolution.*must be a boolean/,
            );
        });
    });
});
