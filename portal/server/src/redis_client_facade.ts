// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { createClient } from 'redis';
import logger from '@lib/logger';

export default class RedisClientFacade {
    private readonly client;

    constructor(redisUrl: string) {
        this.client = createClient({url: redisUrl})
            .on('error', err => console.log('Redis Client Error', err));
    }

    /**
     * Checks if a key exists in a Redis database.
     * @param key The key to check for existence.
     * @returns Promise<boolean> indicating presence of the key.
     */
    async keyExists(key: string): Promise<boolean> {
        try {
            if (!this.client.isReady) {
                await this.client.connect();
            }
            const value = await this.client.EXISTS(key);
            return !!value;
        } catch (error) {
            logger.error({
                message: `Error Redis check: checking the presence of "${key}".`, error
            });
            await this.client.disconnect();
            throw error;
        }
    }

    async close() {
        await this.client.disconnect();
    }
}
