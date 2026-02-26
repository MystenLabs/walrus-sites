// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { describe, it, expect } from "vitest";
import { parsePortalConfigYaml } from "@lib/portal_config";

describe("parsePortalConfigYaml", () => {
    it("parses valid YAML into a PortalYamlConfig object", () => {
        const yaml = `
network: mainnet
site_package: "0x26eb7ee8688da02c5f671679524e379f0b837a12f1d1d799f255b7eea260ad27"
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
        const config = parsePortalConfigYaml(yaml);

        expect(config.network).toBe("mainnet");
        expect(config.site_package).toBe(
            "0x26eb7ee8688da02c5f671679524e379f0b837a12f1d1d799f255b7eea260ad27",
        );
        expect(config.rpc_urls).toEqual([
            { url: "https://rpc.example.com", retries: 2, metric: 100 },
        ]);
        expect(config.b36_domain_resolution).toBe(true);
        expect(config.enable_blocklist).toBe(false);
    });

    it("rejects completely invalid YAML", () => {
        expect(() => parsePortalConfigYaml("{{{{invalid")).toThrow(/Failed to parse YAML/);
    });

    it("rejects YAML that is not an object", () => {
        expect(() => parsePortalConfigYaml("just a string")).toThrow(
            /must be an object at the top level/,
        );
    });

    it("rejects YAML array at top level", () => {
        expect(() => parsePortalConfigYaml("- item1\n- item2")).toThrow(
            /must be an object at the top level/,
        );
    });
});
