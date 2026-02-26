// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { z } from "zod";
import { existsSync, readFileSync } from "fs";
import { parsePriorityUrlList } from "@lib/priority_executor";
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

// --- Merge YAML + env vars → typed config object ---

/**
 * Merges YAML config and env var overrides into a single typed object.
 * All values are converted to their final types before Zod validation.
 * Env vars override YAML values when set.
 */
function buildRawConfig(
    yaml: PortalYamlConfig | null,
    env: Record<string, string | undefined> = process.env as Record<string, string | undefined>,
): Record<string, unknown> {
    const config: Record<string, unknown> = {};

    // Start with YAML values (already typed by the YAML parser)
    if (yaml) {
        config.suinsClientNetwork = yaml.network;
        config.sitePackage = yaml.site_package;
        config.landingPageOidB36 = yaml.landing_page_oid_b36;
        config.rpcUrlList = yaml.rpc_urls;
        config.aggregatorUrlList = yaml.aggregator_urls;
        config.enableBlocklist = yaml.enable_blocklist;
        config.enableAllowlist = yaml.enable_allowlist;
        config.b36DomainResolutionSupport = yaml.b36_domain_resolution;
        if (yaml.premium_rpc_urls !== undefined) config.premiumRpcUrlList = yaml.premium_rpc_urls;
        if (yaml.domain_name_length !== undefined)
            config.portalDomainNameLength = yaml.domain_name_length;
        if (yaml.bring_your_own_domain !== undefined)
            config.bringYourOwnDomain = yaml.bring_your_own_domain;
    }

    // Override with parsed env vars (converted from strings to typed values)
    if (env.SUINS_CLIENT_NETWORK !== undefined)
        config.suinsClientNetwork = env.SUINS_CLIENT_NETWORK;
    if (env.SITE_PACKAGE !== undefined) config.sitePackage = env.SITE_PACKAGE;
    if (env.LANDING_PAGE_OID_B36 !== undefined) config.landingPageOidB36 = env.LANDING_PAGE_OID_B36;
    if (env.RPC_URL_LIST !== undefined) config.rpcUrlList = parsePriorityUrlList(env.RPC_URL_LIST);
    if (env.AGGREGATOR_URL_LIST !== undefined) {
        config.aggregatorUrlList = parsePriorityUrlList(env.AGGREGATOR_URL_LIST, 3);
    } else if (env.AGGREGATOR_URL !== undefined) {
        config.aggregatorUrlList = parsePriorityUrlList(env.AGGREGATOR_URL, 3);
    }
    if (env.PREMIUM_RPC_URL_LIST !== undefined) {
        config.premiumRpcUrlList = parsePriorityUrlList(env.PREMIUM_RPC_URL_LIST);
    }
    if (env.ENABLE_BLOCKLIST !== undefined)
        config.enableBlocklist = env.ENABLE_BLOCKLIST === "true";
    if (env.ENABLE_ALLOWLIST !== undefined)
        config.enableAllowlist = env.ENABLE_ALLOWLIST === "true";
    if (env.B36_DOMAIN_RESOLUTION_SUPPORT !== undefined) {
        config.b36DomainResolutionSupport = env.B36_DOMAIN_RESOLUTION_SUPPORT === "true";
    }
    if (env.BRING_YOUR_OWN_DOMAIN !== undefined)
        config.bringYourOwnDomain = env.BRING_YOUR_OWN_DOMAIN === "true";
    if (env.PORTAL_DOMAIN_NAME_LENGTH !== undefined)
        config.portalDomainNameLength = Number(env.PORTAL_DOMAIN_NAME_LENGTH);

    // Secrets (env-only, no YAML equivalent — contain credentials)
    if (env.EDGE_CONFIG !== undefined) config.edgeConfig = env.EDGE_CONFIG;
    if (env.EDGE_CONFIG_ALLOWLIST !== undefined)
        config.edgeConfigAllowlist = env.EDGE_CONFIG_ALLOWLIST;
    if (env.BLOCKLIST_REDIS_URL !== undefined) config.blocklistRedisUrl = env.BLOCKLIST_REDIS_URL;
    if (env.ALLOWLIST_REDIS_URL !== undefined) config.allowlistRedisUrl = env.ALLOWLIST_REDIS_URL;

    return config;
}

// --- Zod schema (pure validator, no string transforms) ---

const priorityUrlEntrySchema = z.object({
    url: z.string().url(),
    retries: z.number().int().nonnegative(),
    metric: z.number().int(),
});

const configurationSchema = z
    .object({
        edgeConfig: z.string().optional(),
        edgeConfigAllowlist: z.string().optional(),
        enableBlocklist: z.boolean(),
        enableAllowlist: z.boolean(),
        landingPageOidB36: z.string().regex(/^[0-9a-z]+$/i, "Must be a valid base36 string"),
        portalDomainNameLength: z.number().int().positive().optional(),
        premiumRpcUrlList: z.array(priorityUrlEntrySchema).nonempty().optional(),
        rpcUrlList: z.array(priorityUrlEntrySchema).nonempty(),
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
        aggregatorUrlList: z.array(priorityUrlEntrySchema).nonempty(),
        sitePackage: z.string().refine((val) => val.length === 66 && /^0x[0-9a-fA-F]+$/.test(val)),
        b36DomainResolutionSupport: z.boolean(),
        bringYourOwnDomain: z.boolean().default(false),
    })
    /// Extra refinements - Relations between configuration fields:
    // TODO: tighten to XOR — reject if both blocklistRedisUrl and edgeConfig are set
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
    // TODO: tighten to XOR — reject if both allowlistRedisUrl and edgeConfigAllowlist are set
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
    // TODO: reject bringYourOwnDomain together with enableAllowlist/premiumRpcUrlList - the
    // allowlist switches between PREMIUM_RPC_URL_LIST and RPC_URL_LIST based on whether a site is
    // allowlisted. When serving a single domain (BYOD), this differentiation is pointless - just
    // configure the desired nodes in rpc_urls directly.
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
