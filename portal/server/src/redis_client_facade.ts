// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { createClient } from 'redis';
import logger from '@lib/logger';

export default class RedisClientFacade {
    private readonly client;

    constructor(redisUrl: string) {
        this.client = createClient({url: redisUrl})
            .on('error', err => console.log('Redis Client Error', err))
            .on('connect', () => console.log('Redis Client Connected'));
    }

    /**
     * Checks if a key exists in a Redis database.
     * @param key The key to check for existence.
     * @returns Promise<boolean> indicating presence of the key.
     */
    async keyExists(key: string): Promise<boolean> {
        try {
            const value = await this.client.EXISTS(key);
            return !!value;
        } catch (error) {
            logger.error(
				`Error Redis check: checking the presence of "${key}".`, { error }
            );
            throw error;
        }
    }

    async close() {
        await this.client.disconnect();
    }

    async connect(): Promise<void> {
        await this.client.connect();
    }

    async ping(): Promise<boolean> {
        return await this.client.ping() === 'PONG';
    }
}
