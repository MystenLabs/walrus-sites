// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { has } from '@vercel/edge-config';
import BlocklistChecker from "@lib/blocklist_checker";
import { config } from './configuration_loader';
import RedisClientFacade from './redis_client_facade';
import { StorageVariant } from './enums';
import { CheckerBuilder } from './abstract_list_checker';

/**
* Creates a blocklist checker instance based on the deduced storage variant.
*/
export class BlocklistCheckerFactory {
    /// The map of storage variants to their respective blocklist checker constructors.
    /// Lazy instantiation is used to avoid unnecessary initialization of the checkers.
    private static readonly listCheckerVariantsMap = {
        [StorageVariant.VercelEdgeConfig]: () => new VercelEdgeConfigBlocklistChecker(),
        [StorageVariant.Redis]: () => new RedisBlocklistChecker(config.redisUrl),
    } as const; // using const assertion to prevent accidental modification of the map's contents

    /**
    * Builds a blocklist checker instance based on the CheckerBuilder.
    * @returns A blocklist checker instance or undefined if blocklist is disabled.
    */
    static build(): BlocklistChecker | undefined {
        if (!config.enableBlocklist) {
            return undefined;
        }
        return CheckerBuilder.build(this.listCheckerVariantsMap)
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
        return await this.client.isMemberOfSet('walrus-sites-blocklist', id);
    }
}

const blocklistChecker = BlocklistCheckerFactory.build();
export default blocklistChecker;
