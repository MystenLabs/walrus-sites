// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { StorageVariant } from "./enums";
import { config } from "./configuration_loader";
import BlocklistChecker from "@lib/blocklist_checker";
import AllowlistChecker from "./allowlist_checker_interface";

export class CheckerBuilder {
    /**
    * Builds a checker instance based on the deduced storage variant.
    * @returns A checker instance or undefined.
    */
    static buildBlocklistChecker(checkerMap: BlocklistCheckerVariantsMap): BlocklistChecker | undefined {
        const variant = CheckerBuilder.deduceStorageVariant();
        return variant ? checkerMap[variant]() : undefined;
    }

    /**
    * Builds a checker instance based on the deduced storage variant.
    * @returns A checker instance or undefined.
    */
    static buildAllowlistChecker(checkerMap: AllowlistCheckerVariantsMap): AllowlistChecker | undefined {
        const variant = CheckerBuilder.deduceStorageVariant();
        return variant ? checkerMap[variant]() : undefined;
    }

    /**
    * Based on the environment variables set, deduces the storage variant to use.
    * @returns Either the storage variant or undefined.
    */
    private static deduceStorageVariant(): StorageVariant | undefined {
        if (config.edgeConfig) {
            return StorageVariant.VercelEdgeConfig;
        } else if (config.redisUrl) {
            return StorageVariant.Redis;
        }
    }
}

type BlocklistCheckerVariantsMap = {
    [key in StorageVariant]: () => BlocklistChecker;
}

type AllowlistCheckerVariantsMap = {
    [key in StorageVariant]: () => AllowlistChecker;
}
