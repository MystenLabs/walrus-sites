// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { has } from '@vercel/edge-config';
import BlocklistChecker from "@lib/blocklist_checker";
import { config } from 'configuration_loader';
import assert from 'assert';
import { createClient } from 'redis';

/**
 * Supported blocklist storage backends.
 *
 * - VercelEdgeConfig: Looks up a Vercel Edge Config db. Use this for portal deployments on Vercel.
 * - Redis: Provides the flexibility of implementing the blocklist in any platform that can integrate
 * a  Redis database.
 */
enum StorageVariant {
    VercelEdgeConfig = "vercelEdgeConfig",
    Redis = "redis",
}

/**
* Creates a blocklist checker instance based on the deduced storage variant.
*/
class BlocklistCheckerFactory {
    static build(): BlocklistChecker | undefined {
        const variant = BlocklistCheckerFactory.deduceStorageVariant();
        switch (variant) {
            case StorageVariant.VercelEdgeConfig:
                return new VercelEdgeConfigBlocklistChecker();
            case StorageVariant.Redis:
                return new RedisBlocklistChecker();
        }
        return undefined;
    }

    /**
    * Based on the environment variables set, deduces the storage variant to use.
    * @returns Either the storage variant or undefined if blocklist is disabled.
    */
    private static deduceStorageVariant(): StorageVariant | undefined {
        if (!config.enableBlocklist) {
            return
        }
        if (config.edgeConfig) {
            return StorageVariant.VercelEdgeConfig;
        } else if (config.redisUrl) {
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
        assert(
            config.enableBlocklist,
            "ENABLE_BLOCKLIST variable is set to `false`."
        );
        assert(
            config.edgeConfig,
            "EDGE_CONFIG variable is missing."
        )
    }

    async isBlocked(id: string): Promise<boolean> {
        return has(id);
    }
}

/**
* Checks domains/IDs against a Redis blocklist.
*/
class RedisBlocklistChecker implements BlocklistChecker {
    private client;
    private connected = false;

    constructor() {
        if (!config.redisUrl) {
            throw new Error("REDIS_URL variable is missing.");
        }
        this.client = createClient({url: config.redisUrl})
            .on('error', err => console.log('Redis Client Error', err));
    }

    async isBlocked(id: string): Promise<boolean> {
        if (!this.connected) {
            await this.client.connect();
            this.connected = true;
        }
        const value = await this.client.SISMEMBER('walrus-sites-blocklist',id);
        console.log('REDIS IS MEMBER', id, value)
        return !!value;
    }

    async close() {
        await this.client.disconnect();
    }
}

const blocklistChecker = BlocklistCheckerFactory.build();
export default blocklistChecker;
