// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { describe, test, expect } from "bun:test";
import { parse as parseYaml } from "yaml";
import { loadAndValidateConfig } from "./configuration_loader";

const VALID_SITE_PACKAGE = "0x26eb7ee8688da02c5f671679524e379f0b837a12f1d1d799f255b7eea260ad27";

const minimalYaml = `
network: mainnet
site_package: "${VALID_SITE_PACKAGE}"
landing_page_oid_b36: "46f3881sp4r55fc6pcao9t93bieeejl4vr4k2uv8u4wwyx1a93"
enable_blocklist: false
enable_allowlist: false
b36_domain_resolution: true

rpc_urls:
  - url: https://rpc.example.com
    retries: 2
    metric: 100

aggregator_urls:
  - url: https://aggregator.example.com
    retries: 3
    metric: 100
`;

describe("loadAndValidateConfig", () => {
    test("parses YAML and produces a valid Configuration", () => {
        const config = loadAndValidateConfig(parseYaml(minimalYaml), {});
        expect(config.suinsClientNetwork).toBe("mainnet");
        expect(config.sitePackage).toBe(VALID_SITE_PACKAGE);
        expect(config.rpcUrlList).toHaveLength(1);
        expect(config.rpcUrlList[0].url).toBe("https://rpc.example.com");
        expect(config.aggregatorUrlList).toHaveLength(1);
        expect(config.enableBlocklist).toBe(false);
        expect(config.enableAllowlist).toBe(false);
        expect(config.b36DomainResolutionSupport).toBe(true);
        expect(config.bringYourOwnDomain).toBe(false);
    });

    test("env vars override YAML values", () => {
        const config = loadAndValidateConfig(parseYaml(minimalYaml), {
            SUINS_CLIENT_NETWORK: "testnet",
            ENABLE_BLOCKLIST: "true",
            BLOCKLIST_REDIS_URL: "redis://localhost:6379/0",
        });
        expect(config.suinsClientNetwork).toBe("testnet");
        expect(config.enableBlocklist).toBe(true);
        // YAML values still used for non-overridden fields
        expect(config.sitePackage).toBe(VALID_SITE_PACKAGE);
    });

    test("works with null yaml (env-only mode)", () => {
        const config = loadAndValidateConfig(null, {
            SUINS_CLIENT_NETWORK: "mainnet",
            SITE_PACKAGE: VALID_SITE_PACKAGE,
            LANDING_PAGE_OID_B36: "abc123",
            RPC_URL_LIST: "https://rpc.example.com|2|100",
            AGGREGATOR_URL_LIST: "https://agg.example.com|3|100",
            ENABLE_BLOCKLIST: "false",
            ENABLE_ALLOWLIST: "false",
            B36_DOMAIN_RESOLUTION_SUPPORT: "true",
        });
        expect(config.suinsClientNetwork).toBe("mainnet");
        expect(config.rpcUrlList).toEqual([
            { url: "https://rpc.example.com", retries: 2, metric: 100 },
        ]);
    });

    test("handles optional YAML fields", () => {
        const yaml = `
network: mainnet
site_package: "${VALID_SITE_PACKAGE}"
landing_page_oid_b36: "46f3881sp4r55fc6pcao9t93bieeejl4vr4k2uv8u4wwyx1a93"
enable_blocklist: false
enable_allowlist: false
b36_domain_resolution: true
bring_your_own_domain: true
domain_name_length: 21

rpc_urls:
  - url: https://rpc.example.com
    retries: 2
    metric: 100

aggregator_urls:
  - url: https://aggregator.example.com
    retries: 3
    metric: 100
`;
        const config = loadAndValidateConfig(parseYaml(yaml), {});
        expect(config.bringYourOwnDomain).toBe(true);
        expect(config.portalDomainNameLength).toBe(21);
    });

    test("rejects invalid network", () => {
        const yaml = minimalYaml.replace("network: mainnet", "network: devnet");
        expect(() => loadAndValidateConfig(parseYaml(yaml), {})).toThrow(
            /Configuration validation error/,
        );
    });

    test("rejects invalid site_package", () => {
        const yaml = minimalYaml.replace(
            `site_package: "${VALID_SITE_PACKAGE}"`,
            'site_package: "bad"',
        );
        expect(() => loadAndValidateConfig(parseYaml(yaml), {})).toThrow(
            /Configuration validation error/,
        );
    });

    test("rejects enableBlocklist without a storage backend", () => {
        const yaml = minimalYaml.replace("enable_blocklist: false", "enable_blocklist: true");
        expect(() => loadAndValidateConfig(parseYaml(yaml), {})).toThrow(/BLOCKLIST_REDIS_URL/);
    });

    test("rejects enableAllowlist without premiumRpcUrlList", () => {
        const yaml = minimalYaml.replace("enable_allowlist: false", "enable_allowlist: true");
        expect(() =>
            loadAndValidateConfig(parseYaml(yaml), {
                ALLOWLIST_REDIS_URL: "redis://localhost:6379/1",
            }),
        ).toThrow(/PREMIUM_RPC_URL_LIST/);
    });

    test("accepts enableAllowlist with storage backend and premium RPC", () => {
        const yaml = `
network: mainnet
site_package: "${VALID_SITE_PACKAGE}"
landing_page_oid_b36: "46f3881sp4r55fc6pcao9t93bieeejl4vr4k2uv8u4wwyx1a93"
enable_blocklist: false
enable_allowlist: true
b36_domain_resolution: true

rpc_urls:
  - url: https://rpc.example.com
    retries: 2
    metric: 100

premium_rpc_urls:
  - url: https://premium.example.com
    retries: 1
    metric: 50

aggregator_urls:
  - url: https://aggregator.example.com
    retries: 3
    metric: 100
`;
        const config = loadAndValidateConfig(parseYaml(yaml), {
            ALLOWLIST_REDIS_URL: "redis://localhost:6379/1",
        });
        expect(config.enableAllowlist).toBe(true);
        expect(config.premiumRpcUrlList).toHaveLength(1);
    });

    test("multiple RPC urls with priority ordering", () => {
        const yaml = `
network: mainnet
site_package: "${VALID_SITE_PACKAGE}"
landing_page_oid_b36: "46f3881sp4r55fc6pcao9t93bieeejl4vr4k2uv8u4wwyx1a93"
enable_blocklist: false
enable_allowlist: false
b36_domain_resolution: true

rpc_urls:
  - url: https://primary.example.com
    retries: 2
    metric: 100
  - url: https://backup.example.com
    retries: 1
    metric: 200

aggregator_urls:
  - url: https://aggregator.example.com
    retries: 3
    metric: 100
`;
        const config = loadAndValidateConfig(parseYaml(yaml), {});
        expect(config.rpcUrlList).toHaveLength(2);
        expect(config.rpcUrlList[0].metric).toBe(100);
        expect(config.rpcUrlList[1].metric).toBe(200);
    });

    test("blocklist with Redis", () => {
        const yaml = minimalYaml.replace("enable_blocklist: false", "enable_blocklist: true");
        const config = loadAndValidateConfig(parseYaml(yaml), {
            BLOCKLIST_REDIS_URL: "redis://localhost:6379/0",
        });
        expect(config.enableBlocklist).toBe(true);
        expect(config.blocklistRedisUrl).toBe("redis://localhost:6379/0");
    });
});
