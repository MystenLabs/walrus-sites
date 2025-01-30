// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { z } from "zod";

// Define a transformer for string booleans
const stringBoolean = z
  .enum(["true", "false"])
  .transform((val) => val === "true");

const configurationSchema =
	z.preprocess((env: any) => ({
		edgeConfig: env.EDGE_CONFIG,
		edgeConfigAllowlist: env.EDGE_CONFIG_ALLOWLIST,
		enableBlocklist: env.ENABLE_BLOCKLIST,
		enableAllowlist: env.ENABLE_ALLOWLIST,
		landingPageOidB36: env.LANDING_PAGE_OID_B36,
		portalDomainNameLength: env.PORTAL_DOMAIN_NAME_LENGTH,
		premiumRpcUrlList: env.PREMIUM_RPC_URL_LIST,
		rpcUrlList: env.RPC_URL_LIST,
		enableSentry: env.ENABLE_SENTRY,
		sentryAuthToken: env.SENTRY_AUTH_TOKEN,
		sentryDsn: env.SENTRY_DSN,
		sentryTracesSampleRate: env.SENTRY_TRACES_SAMPLE_RATE,
		suinsClientNetwork: env.SUINS_CLIENT_NETWORK,
		blocklistRedisUrl: env.BLOCKLIST_REDIS_URL,
		allowlistRedisUrl: env.ALLOWLIST_REDIS_URL,
	}),
	z.object({
		edgeConfig: z.string().optional(),
	  	edgeConfigAllowlist: z.string().optional(),
	    enableBlocklist: stringBoolean,
		enableAllowlist: stringBoolean,
	    landingPageOidB36: z
			.string()
			.regex(/^[0-9a-z]+$/i, "Must be a valid base36 string"),
	    portalDomainNameLength: z
				.string()
				.optional()
				.transform((val) => (val ? Number(val) : undefined))
				.refine((val) => val === undefined || val > 0, {
					message: "PORTAL_DOMAIN_NAME_LENGTH must be a positive number",
				}),
		premiumRpcUrlList: z.preprocess(
				(val) => typeof val === 'string' ? val.trim().split(',') : val,
				z.array(z.string().url())
			),
	  	rpcUrlList: z.preprocess(
				(val) => typeof val === 'string' ? val.trim().split(',') : val,
				z.array(z.string().url())
			),
		enableSentry: stringBoolean,
	  	sentryAuthToken: z.string().optional(),
		sentryDsn: z.string().optional(),
	  	sentryTracesSampleRate: z
				.string()
				.optional()
				.transform((val) => (val ? Number(val) : undefined))
				.refine((val) => val === undefined || (val >= 0 && val <= 1), {
					message: "SENTRY_TRACES_SAMPLE_RATE must be between 0 and 1",
				}),
	    suinsClientNetwork: z.enum(["testnet", "mainnet"]),
	    blocklistRedisUrl: z.string().optional(),
	    allowlistRedisUrl: z.string().optional(),})
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
  /// Extra refinements - Relations between
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
      if (data.enableSentry) {
        return (
          data.sentryAuthToken &&
          data.sentryDsn &&
          data.sentryTracesSampleRate !== undefined
        );
      }
      return true;
    },
    {
      message: "SENTRY settings are incomplete.",
      path: ["enableSentry"],
    },
  ));

export type Configuration = z.infer<typeof configurationSchema>;

const parsedConfig = configurationSchema.safeParse(process.env);

if (!parsedConfig.success) {
  throw new Error(
    `Configuration validation error: ${parsedConfig.error.message}`,
  );
}

export const config: Configuration = parsedConfig.data;
