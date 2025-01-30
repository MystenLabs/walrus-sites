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
     * Checks if a member exists in a Redis set
     * @param set The name of the set
     * @param member The value to check for membership
     * @returns Promise<boolean> indicating presence in set
     */
    async isMemberOfSet(set: string, member: string): Promise<boolean> {
        try {
            if (!this.client.isReady) {
                await this.client.connect();
            }
            const value = await this.client.SISMEMBER(set, member);
            return !!value;
        } catch (error) {
            logger.error({ message: `Error Redis check: "${member}" contains "${set}"?`, error });
            await this.client.disconnect();
            throw error;
        }
    }

    async close() {
        await this.client.disconnect();
    }
}
