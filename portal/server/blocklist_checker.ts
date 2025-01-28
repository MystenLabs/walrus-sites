// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { has } from '@vercel/edge-config';
import BlocklistChecker from "@lib/blocklist_checker";
import { config } from 'configuration_loader';
import assert from 'assert';

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
* Creates a blocklist checker instance based on the provided storage variant.
*/
class BlocklistCheckerFactory {
    static createBlocklistChecker(variant: StorageVariant): BlocklistChecker {
        switch (variant) {
            case StorageVariant.VercelEdgeConfig:
                return new VercelEdgeConfigBlocklistChecker();
            case StorageVariant.Redis:
                // TODO: Implement Redis blocklist checker.
                throw new Error("Redis blocklist checker is not implemented yet.");
        }
    }
}

/**
 * Checks domains/IDs against Vercel's Edge Config blocklist
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

    async check(id: string): Promise<boolean> {
        return has(id);
    }
}

let blocklistChecker: BlocklistChecker | undefined;
if (config.enableBlocklist) {
    blocklistChecker = BlocklistCheckerFactory.createBlocklistChecker(
        StorageVariant.VercelEdgeConfig
    );
}
export default blocklistChecker;
