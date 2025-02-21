// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { createClient, EdgeConfigClient } from '@vercel/edge-config';
import { config } from './configuration_loader';
import RedisClientFacade from './redis_client_facade';
import { StorageVariant } from './enums';
import AllowlistChecker from './allowlist_checker_interface';

/**
* Creates a allowlist checker instance based on the deduced storage variant.
*/
export class AllowlistCheckerFactory {
    /// The map of storage variants to their respective allowlist checker constructors.
    /// Lazy instantiation is used to avoid unnecessary initialization of the checkers.
    private static readonly listCheckerVariantsMap = {
        [StorageVariant.VercelEdgeConfig]: () => new VercelEdgeConfigAllowlistChecker(),
        [StorageVariant.Redis]: () => new RedisAllowlistChecker(config.allowlistRedisUrl),
    } as const; // using const assertion to prevent accidental modification of the map's contents

    /**
    * Builds a allowlist checker instance.
    * @returns A allowlist checker instance or undefined if allowlist is disabled.
    */
    static build(): AllowlistChecker | undefined {
        if (!config.enableAllowlist) {
            return undefined;
        }
        const variant = this.deduceStorageVariant();
        return variant ? this.listCheckerVariantsMap[variant]() : undefined;
    }

    /**
    * Based on the environment variables set, deduces the storage variant to use.
    * @returns Either the storage variant or undefined.
    */
    private static deduceStorageVariant(): StorageVariant | undefined {
        if (config.edgeConfigAllowlist) {
            return StorageVariant.VercelEdgeConfig;
        } else if (config.allowlistRedisUrl) {
            return StorageVariant.Redis;
        }
    }
}

/**
 * Checks domains/IDs against Vercel's Edge Config allowlist.
 *
 * Validates whether a given identifier is present in the allowlist.
 * Requires allowlist to be enabled via ENABLE_ALLOWLIST environment variable.
 */
class VercelEdgeConfigAllowlistChecker implements AllowlistChecker {
    private edgeConfigAllowlistClient: EdgeConfigClient;

    constructor() {
        if (!config.enableAllowlist){
            throw new Error('ENABLE_ALLOWLIST variable is set to `false`.')
        }
        if (!config.edgeConfigAllowlist) {
            throw new Error("EDGE_CONFIG_ALLOWLIST variable is missing.");
        }

        this.edgeConfigAllowlistClient = createClient(config.edgeConfigAllowlist);
    }

    // edge config does not require initialization, so we do nothing
    async init(): Promise<void> {
        return;
    }

    async isAllowed(id: string): Promise<boolean> {
        return await this.edgeConfigAllowlistClient.has(id);
    }

    // edge config does not support pinging and the client handles the connection, so we always return true
    async ping(): Promise<boolean> {
        return true;
    }
}

/**
* Checks domains/IDs against a Redis allowlist.
*/
class RedisAllowlistChecker implements AllowlistChecker {
    private client: RedisClientFacade;

    constructor(redisUrl?: string) {
        if (!redisUrl) {
            throw new Error("REDIS_URL variable is missing.");
        }
        this.client = new RedisClientFacade(redisUrl);
    }

    async init(): Promise<void> {
        await this.client.connect();
    }

    async isAllowed(id: string): Promise<boolean> {
        return await this.client.keyExists(id);
    }

    async ping(): Promise<boolean> {
        return await this.client.ping();
    }
}

const allowlistChecker = AllowlistCheckerFactory.build();
// Initialize in an IIFE instead of top-level await
(async () => {
    await allowlistChecker?.init();
})();

export default allowlistChecker;
