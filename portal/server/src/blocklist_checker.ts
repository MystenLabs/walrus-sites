// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { has } from '@vercel/edge-config';
import BlocklistChecker from "@lib/blocklist_checker";
import { config } from './configuration_loader';
import RedisClientFacade from './redis_client_facade';
import { StorageVariant } from './enums';

/**
* Creates a blocklist checker instance based on the deduced storage variant.
*/
export class BlocklistCheckerFactory {
    /// The map of storage variants to their respective blocklist checker constructors.
    /// Lazy instantiation is used to avoid unnecessary initialization of the checkers.
    private static readonly listCheckerVariantsMap = {
        [StorageVariant.VercelEdgeConfig]: () => new VercelEdgeConfigBlocklistChecker(),
        [StorageVariant.Redis]: () => new RedisBlocklistChecker(config.blocklistRedisUrl),
    } as const; // using const assertion to prevent accidental modification of the map's contents

    /**
    * Builds a blocklist checker instance.
    * @returns A blocklist checker instance or undefined if blocklist is disabled.
    */
    static build(): BlocklistChecker | undefined {
        if (!config.enableBlocklist) {
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
        if (config.edgeConfig) {
            return StorageVariant.VercelEdgeConfig;
        } else if (config.blocklistRedisUrl) {
            return StorageVariant.Redis;
        }
    }
}

/**
 * Checks domains/IDs against Vercel's Edge Config blocklist.
 *
 * Validates whether a given identifier is present in the blocklist.
 * Requires blocklist to be enabled via ENABLE_ALLOWLIST environment variable.
 */
class VercelEdgeConfigBlocklistChecker implements BlocklistChecker {
    constructor() {
        if (!config.enableBlocklist) {
            throw new Error("ENABLE_BLOCKLIST variable is set to `false`.");
        }
        if (!config.edgeConfig) {
            throw new Error("EDGE_CONFIG variable is missing.");
        }
    }

    async isBlocked(id: string): Promise<boolean> {
        return has(id);
    }

    // edge config does not require initialization, so we do nothing
    async init(): Promise<void> {
        return;
    }

    // edge config does not support pinging, so we always return true
    async ping(): Promise<boolean> {
        return true;
    }
}

/**
* Checks domains/IDs against a Redis blocklist.
*/
class RedisBlocklistChecker implements BlocklistChecker {
    private client: RedisClientFacade;

    constructor(redisUrl?: string) {
        if (!redisUrl) {
            throw new Error("REDIS_URL variable is missing.");
        }
        this.client = new RedisClientFacade(redisUrl);
    }

    async isBlocked(id: string): Promise<boolean> {
        return await this.client.keyExists(id);
    }

    async init(): Promise<void> {
        await this.client.connect();
    }

    async ping(): Promise<boolean> {
        return await this.client.ping();
    }
}

const blocklistChecker = BlocklistCheckerFactory.build();
// Initialize in an IIFE instead of top-level await
(async () => {
    await blocklistChecker?.init();
})();

export default blocklistChecker;
