// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { z } from "zod";
import { parsePriorityUrlList } from "@lib/priority_executor";

// Define a transformer for string booleans
const stringBoolean = z.enum(["true", "false"]).transform((val) => val === "true");

// Custom Zod transformer for priority URL lists (format: URL|RETRIES|PRIORITY)
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
        suinsClientNetwork: env.SUINS_CLIENT_NETWORK, // TODO(alex): rename this to NETWORK
        blocklistRedisUrl: env.BLOCKLIST_REDIS_URL,
        allowlistRedisUrl: env.ALLOWLIST_REDIS_URL,
        aggregatorUrlList: env.AGGREGATOR_URL_LIST,
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
                .refine(
                    // Ensure that the database number is specified and is 0 - this is the blocklist database.
                    (val) => val === undefined || val.endsWith("0"),
                    {
                        message:
                            "BLOCKLIST_REDIS_URL must end with '0' to use the blocklist database.",
                    },
                ),
            allowlistRedisUrl: z
                .string()
                .url({ message: "ALLOWLIST_REDIS_URL is not a valid URL!" })
                .optional()
                .refine(
                    // Ensure that the database number is specified and is 1 - this is the allowlist database.
                    (val) => val === undefined || val.endsWith("1"),
                    {
                        message:
                            "ALLOWLIST_REDIS_URL must end with '1' to use the allowlist database.",
                    },
                ),
            aggregatorUrlList: priorityUrlListSchema,
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

const parsedConfig = configurationSchema.safeParse(process.env);

if (!parsedConfig.success) {
    throw new Error(`Configuration validation error: ${parsedConfig.error.message}`);
}

export const config: Configuration = parsedConfig.data;
