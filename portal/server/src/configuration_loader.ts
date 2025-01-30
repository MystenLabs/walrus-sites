// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

type StringBoolean = "true" | "false";
function isStringBoolean(value: string): value is StringBoolean {
    return value === "true" || value === "false";
}
function toBoolean(value: string): Boolean {
    return value === "true";
}

/**
 * Configuration based on the environment variables.
 */
export type Configuration = {
    edgeConfig?: string;
    edgeConfigAllowlist?: string;
    enableBlocklist: Boolean;
    enableAllowlist: Boolean;
    landingPageOidB36: string;
    portalDomainNameLength?: number;
    premiumRpcUrlList: string[];
    rpcUrlList: string[];
    enableSentry: Boolean;
    sentryAuthToken?: string;
    sentryDsn?: string;
    sentryTracesSampleRate?: number;
    suinsClientNetwork: 'testnet' | 'mainnet'
    blocklistRedisUrl?: string;
    allowlistRedisUrl?: string;
};

/**
 * A utility class that makes it safe to load the environment
 * variables of the project.
 * By using this class, we can ensure that the environment
 * variables are loaded correctly and their data types are
 * correct.
 */
class ConfigurationLoader {
    get config(): Configuration {
        return {
            enableBlocklist: this.loadEnableBlocklist(),
            enableAllowlist: this.loadEnableAllowlist(),
            edgeConfig: this.loadEdgeConfig(),
            edgeConfigAllowlist: this.loadEdgeConfigAllowlist(),
            landingPageOidB36: this.loadLandingPageOidB36(),
            portalDomainNameLength: this.loadPortalDomainNameLength(),
            premiumRpcUrlList: this.loadPremiumRpcUrlList(),
            rpcUrlList: this.loadRpcUrlList(),
            enableSentry: this.loadEnableSentry(),
            sentryAuthToken: this.loadSentryAuthToken(),
            sentryDsn: this.loadSentryDsn(),
            sentryTracesSampleRate: this.loadSentryTracesSampleRate(),
            suinsClientNetwork: this.loadSuinsClientNetwork(),
            blocklistRedisUrl: this.loadBlocklistRedisUrl(),
            allowlistRedisUrl: this.loadAllowlistRedisUrl(),
        };
    }

    private loadEdgeConfig(): string | undefined {
        return process.env.EDGE_CONFIG
    }

    private loadEdgeConfigAllowlist(): string | undefined {
        return this.loadEnableAllowlist() ? process.env.EDGE_CONFIG_ALLOWLIST : undefined;
    }

    private loadEnableBlocklist(): Boolean {
        if (!process.env.ENABLE_BLOCKLIST) {
            throw new Error("Missing ENABLE_BLOCKLIST environment variable.");
        }
        const enable = process.env.ENABLE_BLOCKLIST.toLowerCase();
        if (!isStringBoolean(enable)) {
            throw new Error('ENABLE_BLOCKLIST must be "true" or "false".');
        }
        const value = toBoolean(enable)
        if(value && !this.loadBlocklistRedisUrl() && !this.loadEdgeConfig()) {
            throw new Error("ENABLE_BLOCKLIST is set to `true` but neither REDIS_URL nor EDGE_CONFIG is set.")
        }
        return value
    }

    private loadEnableAllowlist(): Boolean {
        if (!process.env.ENABLE_ALLOWLIST) {
            throw new Error("Missing ENABLE_ALLOWLIST environment variable.");
        }
        const enable = process.env.ENABLE_ALLOWLIST.toLowerCase();
        if (!isStringBoolean(enable)) {
            throw new Error('ENABLE_ALLOWLIST must be "true" or "false".');
        }
        const value = toBoolean(enable)
        if(value && !this.loadAllowlistRedisUrl() && !this.loadEdgeConfigAllowlist()) {
            throw new Error("ENABLE_ALLOWLIST is set to `true` but neither REDIS_URL nor EDGE_CONFIG_ALLOWLIST is set.")
        }
        return value
    }

    private loadLandingPageOidB36(): string {
        const pageOidB36 = process.env.LANDING_PAGE_OID_B36;
        if (!pageOidB36) {
            throw new Error("Missing LANDING_PAGE_OID_B36 environment variable.");
        }
        const base36Pattern = /^[0-9a-z]+$/i;
        if (!base36Pattern.test(pageOidB36)) {
            throw new Error("LANDING_PAGE_OID_B36 must be a valid base36 string.");
        }
        return pageOidB36;
    }

    private loadPortalDomainNameLength(): number | undefined {
        const portalDomainNameLength = process.env.PORTAL_DOMAIN_NAME_LENGTH;
        if (!portalDomainNameLength) {
            return undefined;
        }
        if (portalDomainNameLength && Number(portalDomainNameLength) <= 0) {
            throw new Error("PORTAL_DOMAIN_NAME_LENGTH must be positive number.");
        }
        return Number(portalDomainNameLength);
    }

    private loadPremiumRpcUrlList(): string[] {
        const premiumRpcUrlListString = process.env.PREMIUM_RPC_URL_LIST;
        if (!premiumRpcUrlListString) {
            throw new Error("Missing PREMIUM_RPC_URL_LIST environment variable.");
        }
        const premiumRpcUrlList = premiumRpcUrlListString.split(",");
        if (premiumRpcUrlList.length <= 0) {
            throw new Error("PREMIUM_RPC_URL_LIST must not be empty.");
        }
        return premiumRpcUrlList;
    }

    private loadRpcUrlList(): string[] {
        const rpcUrlListString = process.env.RPC_URL_LIST;
        if (!rpcUrlListString) {
            throw new Error("Missing RPC_URL_LIST environment variable.");
        }
        const rpcUrlList = rpcUrlListString.trim().split(",");
        if (rpcUrlList.length <= 0) {
            throw new Error("RPC_URL_LIST must not be empty.");
        }
        return rpcUrlList;
    }

    private loadEnableSentry(): Boolean {
        if (!process.env.ENABLE_SENTRY) {
            throw new Error("Missing ENABLE_SENTRY environment variable.");
        }
        const enable = process.env.ENABLE_SENTRY.toLowerCase();
        if (!isStringBoolean(enable)) {
            throw new Error('ENABLE_SENTRY must be "true" or "false".');
        }
        return toBoolean(enable);
    }

    private loadSentryAuthToken(): string | undefined {
        if (this.loadEnableSentry()) {
            const authToken = process.env.SENTRY_AUTH_TOKEN;
            if (!authToken) {
                throw new Error("Missing SENTRY_AUTH_TOKEN environment variable.");
            }
            return authToken;
        }
    }

    private loadSentryDsn(): string | undefined {
        if (this.loadEnableSentry()) {
            const dsn = process.env.SENTRY_DSN;
            if (!dsn) {
                throw new Error("Missing SENTRY_DSN environment variable.");
            }
            return dsn;
        }
    }

    private loadSentryTracesSampleRate(): number | undefined {
        if (this.loadEnableSentry()) {
            const tracesSampleRate = process.env.SENTRY_TRACES_SAMPLE_RATE;
            if (!tracesSampleRate) {
                throw new Error("Missing SENTRY_TRACES_SAMPLE_RATE environment variable.");
            }
            const rate = Number(tracesSampleRate);
            if (rate < 0 || rate > 1) {
                throw new Error("SENTRY_TRACES_SAMPLE_RATE must be a number between 0 and 1.");
            }
            return rate;
        }
    }

    private loadSuinsClientNetwork(): 'testnet' | 'mainnet' {
        const suinsClientNetworkValue = process.env.SUINS_CLIENT_NETWORK;
        if (suinsClientNetworkValue) {
            if (suinsClientNetworkValue == 'testnet' || suinsClientNetworkValue == 'mainnet') {
                return suinsClientNetworkValue
            }
            throw new Error(
                "Incorrect SUINS_CLIENT_NETWORK value! Should be either 'testnet' or 'mainnet'"
            )
        }
        throw new Error("No SUINS_CLIENT_NETWORK variable set!")
    }

    private loadBlocklistRedisUrl(): string | undefined {
        return process.env.BLOCKLIST_REDIS_URL;
    }

    private loadAllowlistRedisUrl(): string | undefined {
        return process.env.ALLOWLIST_REDIS_URL;
    }
}

export const config = new ConfigurationLoader().config;
