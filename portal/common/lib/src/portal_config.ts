// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { parse as parseYaml } from "yaml";
import type { PriorityUrl } from "./priority_executor";

// --- Types ---

export interface PortalYamlConfig {
    network: "testnet" | "mainnet";
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

// --- Validation helpers ---

function assertString(value: unknown, field: string): string {
    if (typeof value !== "string") {
        throw new Error(`'${field}' must be a string, got ${typeof value}`);
    }
    return value;
}

function assertBoolean(value: unknown, field: string): boolean {
    if (typeof value !== "boolean") {
        throw new Error(`'${field}' must be a boolean, got ${typeof value}`);
    }
    return value;
}

function assertNumber(value: unknown, field: string): number {
    if (typeof value !== "number" || !Number.isFinite(value)) {
        throw new Error(`'${field}' must be a number, got ${typeof value}`);
    }
    return value;
}

function validateUrl(url: string, context: string): void {
    try {
        new URL(url);
    } catch {
        throw new Error(`Invalid URL in '${context}': "${url}"`);
    }
}

function validatePriorityUrlArray(value: unknown, field: string): PriorityUrl[] {
    if (!Array.isArray(value)) {
        throw new Error(`'${field}' must be an array`);
    }
    if (value.length === 0) {
        throw new Error(`'${field}' must not be empty`);
    }
    return value.map((entry, i) => {
        if (typeof entry !== "object" || entry === null) {
            throw new Error(`'${field}[${i}]' must be an object`);
        }
        const obj = entry as Record<string, unknown>;
        const url = assertString(obj.url, `${field}[${i}].url`);
        validateUrl(url, `${field}[${i}].url`);
        const retries = assertNumber(obj.retries, `${field}[${i}].retries`);
        if (!Number.isInteger(retries) || retries < 0) {
            throw new Error(
                `'${field}[${i}].retries' must be a non-negative integer, got ${retries}`,
            );
        }
        const metric = assertNumber(obj.metric, `${field}[${i}].metric`);
        if (!Number.isInteger(metric)) {
            throw new Error(`'${field}[${i}].metric' must be an integer, got ${metric}`);
        }
        return { url, retries, metric };
    });
}

// --- Parser ---

/**
 * Parses and validates a YAML string into a PortalYamlConfig.
 *
 * @throws Error if the YAML is malformed or any field fails validation
 */
export function parsePortalConfigYaml(yamlContent: string): PortalYamlConfig {
    let raw: unknown;
    try {
        raw = parseYaml(yamlContent);
    } catch (e) {
        throw new Error(
            `Failed to parse YAML: ${e instanceof Error ? e.message : String(e)}`,
        );
    }

    if (typeof raw !== "object" || raw === null) {
        throw new Error("Portal config YAML must be an object at the top level");
    }

    const doc = raw as Record<string, unknown>;

    // Required string fields
    const network = assertString(doc.network, "network");
    if (network !== "testnet" && network !== "mainnet") {
        throw new Error(`'network' must be "testnet" or "mainnet", got "${network}"`);
    }

    const site_package = assertString(doc.site_package, "site_package");
    if (!/^0x[0-9a-fA-F]{64}$/.test(site_package)) {
        throw new Error(
            `'site_package' must match 0x + 64 hex chars, got "${site_package}"`,
        );
    }

    const landing_page_oid_b36 = assertString(doc.landing_page_oid_b36, "landing_page_oid_b36");
    if (!/^[0-9a-z]+$/i.test(landing_page_oid_b36)) {
        throw new Error(
            `'landing_page_oid_b36' must be a valid base36 string, got "${landing_page_oid_b36}"`,
        );
    }

    // Required URL arrays
    const rpc_urls = validatePriorityUrlArray(doc.rpc_urls, "rpc_urls");
    const aggregator_urls = validatePriorityUrlArray(doc.aggregator_urls, "aggregator_urls");

    // Optional URL array
    let premium_rpc_urls: PriorityUrl[] | undefined;
    if (doc.premium_rpc_urls !== undefined) {
        premium_rpc_urls = validatePriorityUrlArray(doc.premium_rpc_urls, "premium_rpc_urls");
    }

    // Required booleans
    const b36_domain_resolution = assertBoolean(doc.b36_domain_resolution, "b36_domain_resolution");
    const enable_blocklist = assertBoolean(doc.enable_blocklist, "enable_blocklist");
    const enable_allowlist = assertBoolean(doc.enable_allowlist, "enable_allowlist");

    // Optional fields
    let domain_name_length: number | undefined;
    if (doc.domain_name_length !== undefined) {
        domain_name_length = assertNumber(doc.domain_name_length, "domain_name_length");
        if (!Number.isInteger(domain_name_length) || domain_name_length <= 0) {
            throw new Error(
                `'domain_name_length' must be a positive integer, got ${domain_name_length}`,
            );
        }
    }

    let bring_your_own_domain: boolean | undefined;
    if (doc.bring_your_own_domain !== undefined) {
        bring_your_own_domain = assertBoolean(doc.bring_your_own_domain, "bring_your_own_domain");
    }

    return {
        network: network as "testnet" | "mainnet",
        site_package,
        landing_page_oid_b36,
        rpc_urls,
        premium_rpc_urls,
        aggregator_urls,
        domain_name_length,
        b36_domain_resolution,
        bring_your_own_domain,
        enable_blocklist,
        enable_allowlist,
    };
}
