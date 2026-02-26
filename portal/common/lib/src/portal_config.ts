// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { parse as parseYaml } from "yaml";
import type { PriorityUrl } from "./priority_executor";

/**
 * Shape of the portal YAML configuration file.
 * Full validation is handled downstream by the Zod schema in configuration_schema.
 */
export interface PortalYamlConfig {
    network: string;
    site_package: string;
    landing_page_oid_b36: string;
    rpc_urls: PriorityUrl[];
    premium_rpc_urls?: PriorityUrl[];
    aggregator_urls: PriorityUrl[];
    domain_name_length?: number;
    b36_domain_resolution: boolean;
    bring_your_own_domain?: boolean;
    enable_blocklist: boolean;
    enable_allowlist: boolean;
}

/**
 * Parses a YAML string into a PortalYamlConfig.
 * Only performs basic structural checks; full validation is done by the Zod schema.
 *
 * @throws Error if the YAML is malformed or not an object
 */
export function parsePortalConfigYaml(yamlContent: string): PortalYamlConfig {
    let raw: unknown;
    try {
        raw = parseYaml(yamlContent);
    } catch (e) {
        throw new Error(`Failed to parse YAML: ${e instanceof Error ? e.message : String(e)}`);
    }

    if (typeof raw !== "object" || raw === null || Array.isArray(raw)) {
        throw new Error("Portal config YAML must be an object at the top level");
    }

    return raw as PortalYamlConfig;
}
