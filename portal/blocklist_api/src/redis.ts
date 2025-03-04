// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { createClient } from 'redis';

class RedisClientFacade {
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
    async exists(key: string): Promise<boolean> {
        try {
        	const open = this.client.isReady
        	if (!!!open) {
         		console.log(`Client is ${open ? 'not' :''}open`)
        		await this.client.connect();
        	}
            const value = await this.client.EXISTS(key);
            return !!value;
        } catch (error) {
            throw error;
        }
    }

    /**
    * Sets a key in a Redis database.
    * @param key The key to set.
    * @returns Promise<void> indicating completion of the operation.
    */
    async set(key: string): Promise<void> {
        try {
            await this.client.SET(key, "");
        } catch (error) {
            throw error;
        }
    }

    /**
    * Deletes a key from a Redis database.
    * @param key The key to delete.
    * @returns Promise<void> indicating completion of the operation.
    */
    async delete(key: string): Promise<void> {
        try {
            await this.client.DEL(key);
        } catch (error) {
            throw error;
        }
    }

    async connect(): Promise<void> {
        try {
            await this.client.connect();
        } catch (error) {
            throw error;
        }
    }

    /**
    * Closes the Redis client connection.
    * @returns Promise<void> indicating completion of the operation.
    */
    async close(): Promise<void> {
	   	if (this.client.isReady) {
	  		await this.client.disconnect();
	   	}
    }

    /**
    * Checks the Redis server uptime status.
    * @returns Promise<boolean> indicating the server's PONG response.
    */
    async ping(): Promise<boolean> {
        return await this.client.ping() === 'PONG';
    }
}

const redisUrl = process.env.REDIS_WRITE_URL;
if (!redisUrl) {
	throw new Error("REDIS_WRITE_URL is not set.");
}
if (!redisUrl.endsWith('0')) {
	throw new Error("The blocklist database should have a `0` index.");
}
const redisClient = new RedisClientFacade(
	redisUrl
);
(async () => {
    await redisClient.connect();
})();

export default redisClient;
