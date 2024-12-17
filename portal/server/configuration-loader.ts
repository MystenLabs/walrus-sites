// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

type StringBoolean = 'true' | 'false';
function isStringBoolean(value: string): value is StringBoolean {
    return value === 'true' || value === 'false';
}
function toBoolean(value: string): Boolean {
    return value === 'true';
}


/**
* Configuration based on the environment variables.
*/
export type Configuration = {
    edgeConfig?: string;
    enableBlocklist: Boolean;
    landingPageOidB36: string;
    portalDomainNameLength?: number;
    premiumRpcUrlList: string[];
    rpcUrlList: string[];
    enableSentry: Boolean;
    sentryAuthToken?: string;
    sentryDsn?: string;
}

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
            edgeConfig: this.loadEdgeConfig(),
            landingPageOidB36: this.loadLandingPageOidB36(),
            portalDomainNameLength: this.loadPortalDomainNameLength(),
            premiumRpcUrlList: this.loadPremiumRpcUrlList(),
            rpcUrlList: this.loadRpcUrlList(),
            enableSentry: this.loadEnableSentry(),
            sentryAuthToken: this.loadSentryAuthToken(),
            sentryDsn: this.loadSentryDsn()
        }
    }

    private loadEdgeConfig(): string | undefined {
        return this.loadEnableBlocklist() ? process.env.EDGE_CONFIG : undefined
    }

    private loadEnableBlocklist(): Boolean {
        if (!process.env.ENABLE_BLOCKLIST) {
            throw new Error('Missing ENABLE_BLOCKLIST environment variable.')
        }
        const enable = process.env.ENABLE_BLOCKLIST.toLowerCase()
        if (!isStringBoolean(enable)) {
            throw new Error('ENABLE_BLOCKLIST must be "true" or "false".')
        }
        return toBoolean(enable)
    }

    private loadLandingPageOidB36(): string {
        const pageOidB36 = process.env.LANDING_PAGE_OID_B36
        if (!pageOidB36) {
            throw new Error('Missing LANDING_PAGE_OID_B36 environment variable.')
        }
        const base36Pattern = /^[0-9a-z]+$/i
        if (!base36Pattern.test(pageOidB36)) {
            throw new Error('LANDING_PAGE_OID_B36 must be a valid base36 string.')
        }
        return pageOidB36
    }

    private loadPortalDomainNameLength(): number | undefined {
        const portalDomainNameLength = process.env.PORTAL_DOMAIN_NAME_LENGTH
        if (!portalDomainNameLength) {
            return undefined
        }
        if (portalDomainNameLength && Number(portalDomainNameLength) <= 0) {
            throw new Error('PORTAL_DOMAIN_NAME_LENGTH must be positive number.')
        }
        return Number(portalDomainNameLength)
    }

    private loadPremiumRpcUrlList(): string[] {
        const premiumRpcUrlListString = process.env.PREMIUM_RPC_URL_LIST
        if (!premiumRpcUrlListString) {
           throw new Error('Missing PREMIUM_RPC_URL_LIST environment variable.')
        }
        const premiumRpcUrlList = premiumRpcUrlListString.split(',')
        if (premiumRpcUrlList.length <= 0) {
            throw new Error('PREMIUM_RPC_URL_LIST must not be empty.')
        }
        return premiumRpcUrlList
    }

    private loadRpcUrlList(): string[] {
        const rpcUrlListString = process.env.RPC_URL_LIST
        if (!rpcUrlListString) {
            throw new Error('Missing RPC_URL_LIST environment variable.')
        }
        const rpcUrlList = rpcUrlListString.trim().split(',')
        if (rpcUrlList.length <= 0) {
            throw new Error('RPC_URL_LIST must not be empty.')
        }
        return rpcUrlList
    }

    private loadEnableSentry(): Boolean {
        if (!process.env.ENABLE_SENTRY) {
            throw new Error('Missing ENABLE_SENTRY environment variable.')
        }
        const enable = process.env.ENABLE_SENTRY.toLowerCase()
        if (!isStringBoolean(enable)) {
            throw new Error('ENABLE_SENTRY must be "true" or "false".')
        }
        return toBoolean(enable)
    }

    private loadSentryAuthToken(): string | undefined {
        if (this.loadEnableSentry()) {
            const authToken = process.env.SENTRY_AUTH_TOKEN
            if (!authToken) {
                throw new Error('Missing SENTRY_AUTH_TOKEN environment variable.')
            }
            return authToken
        }
    }

    private loadSentryDsn(): string | undefined {
        if (this.loadEnableSentry()) {
            const dsn = process.env.SENTRY_DSN
            if (!dsn) {
                throw new Error('Missing SENTRY_DSN environment variable.')
            }
            return dsn
        }
    }
}

export const config = new ConfigurationLoader().config;
