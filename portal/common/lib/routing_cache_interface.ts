// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { Routes, Empty } from "./types";

/**
 * Abstract class for a cache that stores Routes objects,
 * to be able to swap out different caches between
 * different portal implementations.
 */
export interface RoutingCacheInterface {
    get(key: string): Promise<Routes | Empty | undefined>;
    /// A note on Empty:
    /// This is used to indicate that the cache does not have a value for the key.
    /// It is used to differentiate between a cache miss and a cache hit with an empty value.
    /// Cache hit with empty values should not trigger a fetch from the fullnode.
    set(key: string, value: Routes | Empty): Promise<void>;
    delete(key: string): Promise<void>;
}
