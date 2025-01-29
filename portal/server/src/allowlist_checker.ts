// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { createClient, EdgeConfigClient } from '@vercel/edge-config';
import { config } from './configuration_loader';
import RedisClientFacade from './redis_client_facade';
import { StorageVariant } from './enums';
import { CheckerBuilder } from './list_checker_builder';
import AllowlistChecker from './allowlist_checker_interface';

/**
* Creates a allowlist checker instance based on the deduced storage variant.
*/
export class AllowlistCheckerFactory {
    /// The map of storage variants to their respective allowlist checker constructors.
    /// Lazy instantiation is used to avoid unnecessary initialization of the checkers.
    private static readonly listCheckerVariantsMap = {
        [StorageVariant.VercelEdgeConfig]: () => new VercelEdgeConfigAllowlistChecker(),
        [StorageVariant.Redis]: () => new RedisAllowlistChecker(config.redisUrl),
    } as const; // using const assertion to prevent accidental modification of the map's contents

    /**
    * Builds a allowlist checker instance based on the CheckerBuilder.
    * @returns A allowlist checker instance or undefined if allowlist is disabled.
    */
    static build(): AllowlistChecker | undefined {
        if (!config.enableAllowlist) {
            return undefined;
        }
        return CheckerBuilder.buildAllowlistChecker(this.listCheckerVariantsMap)
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

    async isAllowed(id: string): Promise<boolean> {
        return await this.edgeConfigAllowlistClient.has(id);
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

    async isAllowed(id: string): Promise<boolean> {
        return await this.client.isMemberOfSet('walrus-sites-allowlist', id);
    }
}

const allowlistChecker = AllowlistCheckerFactory.build();
export default allowlistChecker;
