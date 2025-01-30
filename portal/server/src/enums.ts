// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

/**
 * Supported storage backends:
 * - VercelEdgeConfig: Uses Vercel Edge Config database. Only for portal deployments on Vercel.
 * - Redis: Flexible for any platform capable of integrating with a Redis database.
 */
export enum StorageVariant {
    VercelEdgeConfig = "vercelEdgeConfig",
    Redis = "redis",
}
