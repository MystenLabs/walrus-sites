// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { z } from "zod";
import { existsSync, readFileSync } from "fs";
import { parsePriorityUrlList, type PriorityUrl } from "@lib/priority_executor";
import { parsePortalConfigYaml, type PortalYamlConfig } from "@lib/portal_config";
import logger from "@lib/logger";

// --- YAML loading ---

function loadYamlConfig(): PortalYamlConfig | null {
    const configPath = process.env.PORTAL_CONFIG || "portal-config.yaml";
    if (!existsSync(configPath)) {
        if (process.env.PORTAL_CONFIG) {
            throw new Error(`PORTAL_CONFIG points to "${configPath}" but the file does not exist`);
        }
        logger.info("No portal-config.yaml found, using env vars only");
        return null;
    }
    logger.info(`Loading portal config from ${configPath}`);
    const content = readFileSync(configPath, "utf-8");
    return parsePortalConfigYaml(content);
}

/**
 * Serializes a PriorityUrl[] back to pipe-delimited format (URL|RETRIES|METRIC,...).
 * Used to convert YAML URL arrays into the string format expected by the Zod schema.
 */
function priorityUrlsToPipeString(urls: PriorityUrl[]): string {
    return urls.map((u) => `${u.url}|${u.retries}|${u.metric}`).join(",");
}

// --- Merge YAML + env vars → raw env-like object ---

function buildRawConfig(yaml: PortalYamlConfig | null): Record<string, string | undefined> {
    // Start with YAML values (converted to the string format the Zod schema expects)
    const fromYaml: Record<string, string | undefined> = {};
    if (yaml) {
        fromYaml.SUINS_CLIENT_NETWORK = yaml.network;
        fromYaml.SITE_PACKAGE = yaml.site_package;
        fromYaml.LANDING_PAGE_OID_B36 = yaml.landing_page_oid_b36;
        fromYaml.RPC_URL_LIST = priorityUrlsToPipeString(yaml.rpc_urls);
        fromYaml.AGGREGATOR_URL_LIST = priorityUrlsToPipeString(yaml.aggregator_urls);
        fromYaml.B36_DOMAIN_RESOLUTION_SUPPORT = String(yaml.b36_domain_resolution);
        fromYaml.ENABLE_BLOCKLIST = String(yaml.enable_blocklist);
        fromYaml.ENABLE_ALLOWLIST = String(yaml.enable_allowlist);

        if (yaml.premium_rpc_urls) {
            fromYaml.PREMIUM_RPC_URL_LIST = priorityUrlsToPipeString(yaml.premium_rpc_urls);
        }
        if (yaml.domain_name_length !== undefined) {
            fromYaml.PORTAL_DOMAIN_NAME_LENGTH = String(yaml.domain_name_length);
        }
        if (yaml.bring_your_own_domain !== undefined) {
            fromYaml.BRING_YOUR_OWN_DOMAIN = String(yaml.bring_your_own_domain);
        }
    }

    // Env vars that can override YAML (non-secret portal config)
    const overrideKeys = [
        "SUINS_CLIENT_NETWORK",
        "SITE_PACKAGE",
        "LANDING_PAGE_OID_B36",
        "RPC_URL_LIST",
        "PREMIUM_RPC_URL_LIST",
        "AGGREGATOR_URL_LIST",
        "AGGREGATOR_URL", // legacy fallback
        "B36_DOMAIN_RESOLUTION_SUPPORT",
        "ENABLE_BLOCKLIST",
        "ENABLE_ALLOWLIST",
        "PORTAL_DOMAIN_NAME_LENGTH",
        "BRING_YOUR_OWN_DOMAIN",
    ];

    // Env vars that are secrets (no YAML equivalent)
    const secretKeys = [
        "EDGE_CONFIG",
        "EDGE_CONFIG_ALLOWLIST",
        "BLOCKLIST_REDIS_URL",
        "ALLOWLIST_REDIS_URL",
    ];

    const merged: Record<string, string | undefined> = { ...fromYaml };

    // Layer env var overrides: if set, they win over YAML
    for (const key of overrideKeys) {
        if (process.env[key] !== undefined) {
            merged[key] = process.env[key];
        }
    }

    // Layer secrets from env vars (these have no YAML equivalent)
    for (const key of secretKeys) {
        if (process.env[key] !== undefined) {
            merged[key] = process.env[key];
        }
    }

    return merged;
}

// --- Zod schema (unchanged validation logic) ---

const stringBoolean = z.enum(["true", "false"]).transform((val) => val === "true");

const priorityUrlListSchema = z.string().transform((val, ctx) => {
    try {
        return parsePriorityUrlList(val);
    } catch (e) {
        ctx.addIssue({
            code: z.ZodIssueCode.custom,
            message: e instanceof Error ? e.message : "Invalid priority URL list",
        });
        return z.NEVER;
    }
});

const configurationSchema = z.preprocess(
    (env: any) => ({
        edgeConfig: env.EDGE_CONFIG,
        edgeConfigAllowlist: env.EDGE_CONFIG_ALLOWLIST,
        enableBlocklist: env.ENABLE_BLOCKLIST,
        enableAllowlist: env.ENABLE_ALLOWLIST,
        landingPageOidB36: env.LANDING_PAGE_OID_B36,
        portalDomainNameLength: env.PORTAL_DOMAIN_NAME_LENGTH,
        premiumRpcUrlList: env.PREMIUM_RPC_URL_LIST,
        rpcUrlList: env.RPC_URL_LIST,
        suinsClientNetwork: env.SUINS_CLIENT_NETWORK,
        blocklistRedisUrl: env.BLOCKLIST_REDIS_URL,
        allowlistRedisUrl: env.ALLOWLIST_REDIS_URL,
        aggregatorUrlList: env.AGGREGATOR_URL_LIST || env.AGGREGATOR_URL,
        sitePackage: env.SITE_PACKAGE,
        b36DomainResolutionSupport: env.B36_DOMAIN_RESOLUTION_SUPPORT,
        bringYourOwnDomain: env.BRING_YOUR_OWN_DOMAIN,
    }),
    z
        .object({
            edgeConfig: z.string().optional(),
            edgeConfigAllowlist: z.string().optional(),
            enableBlocklist: stringBoolean,
            enableAllowlist: stringBoolean,
            landingPageOidB36: z.string().regex(/^[0-9a-z]+$/i, "Must be a valid base36 string"),
            portalDomainNameLength: z
                .string()
                .optional()
                .transform((val) => (val ? Number(val) : undefined))
                .refine((val) => val === undefined || val > 0, {
                    message: "PORTAL_DOMAIN_NAME_LENGTH must be a positive number",
                }),
            premiumRpcUrlList: priorityUrlListSchema.optional(),
            rpcUrlList: priorityUrlListSchema,
            suinsClientNetwork: z.enum(["testnet", "mainnet"]),
            blocklistRedisUrl: z
                .string()
                .url({ message: "BLOCKLIST_REDIS_URL is not a valid URL!" })
                .optional()
                .refine((val) => val === undefined || val.endsWith("0"), {
                    message: "BLOCKLIST_REDIS_URL must end with '0' to use the blocklist database.",
                }),
            allowlistRedisUrl: z
                .string()
                .url({ message: "ALLOWLIST_REDIS_URL is not a valid URL!" })
                .optional()
                .refine((val) => val === undefined || val.endsWith("1"), {
                    message: "ALLOWLIST_REDIS_URL must end with '1' to use the allowlist database.",
                }),
            aggregatorUrlList: z.string().transform((val) => parsePriorityUrlList(val, 3)),
            sitePackage: z
                .string()
                .refine((val) => val.length === 66 && /^0x[0-9a-fA-F]+$/.test(val)),
            b36DomainResolutionSupport: stringBoolean,
            bringYourOwnDomain: stringBoolean.optional().transform((val) => (val ? val : false)),
        })
        /// Extra refinements - Relations between environment variables:
        .refine(
            (data) => {
                if (data.enableBlocklist) {
                    return data.blocklistRedisUrl || data.edgeConfig;
                }
                return true;
            },
            {
                message:
                    "ENABLE_BLOCKLIST is true but neither BLOCKLIST_REDIS_URL nor EDGE_CONFIG is set.",
                path: ["enableBlocklist"],
            },
        )
        .refine(
            (data) => {
                if (data.enableAllowlist) {
                    return data.allowlistRedisUrl || data.edgeConfigAllowlist;
                }
                return true;
            },
            {
                message:
                    "ENABLE_ALLOWLIST is true but neither ALLOWLIST_REDIS_URL nor EDGE_CONFIG_ALLOWLIST is set.",
                path: ["enableAllowlist"],
            },
        )
        .refine(
            (data) => {
                if (data.enableAllowlist) {
                    return data.premiumRpcUrlList && data.premiumRpcUrlList.length > 0;
                }
                return true;
            },
            {
                message: "ENABLE_ALLOWLIST is true but PREMIUM_RPC_URL_LIST is not set.",
                path: ["premiumRpcUrlList"],
            },
        ),
);

export type Configuration = z.infer<typeof configurationSchema>;

// --- Load and validate ---

const yamlConfig = loadYamlConfig();
const rawConfig = buildRawConfig(yamlConfig);
const parsedConfig = configurationSchema.safeParse(rawConfig);

if (!parsedConfig.success) {
    throw new Error(`Configuration validation error: ${parsedConfig.error.message}`);
}

export const config: Configuration = parsedConfig.data;
