// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0
import { StorageVariant } from "./enums";
import { ListChecker, CheckerMap as ListCheckerVariantsMap} from "./types";
import { config } from "./configuration_loader";

export class CheckerBuilder {
    /**
    * Builds a checker instance based on the deduced storage variant.
    * @returns A checker instance or undefined.
    */
    static build(checkerMap: ListCheckerVariantsMap): ListChecker | undefined {
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
