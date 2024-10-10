// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { RoutingCacheInterface } from "@lib/routing_cache_interface";
import { Routes, isRoutes, Empty, isEmpty } from "@lib/types";

/**
 * ServiceWorkerRoutingCache is a class that implements the RoutingCacheInterface.
 * It provides methods to initialize a cache, get, set, and delete routing cache entries.
 * The cache is used to store routing information for service workers.
 */
export class ServiceWorkerRoutingCache implements RoutingCacheInterface {
    /// The IndexedDB database used to store the cache.
    private cache: IDBDatabase;
    /// Used to ensure that the cache is initialized before any operations are performed.
    private initialized?: Promise<unknown>;

    /// Initializes the routing cache. Using this instead of a
    /// constructor because the constructor cannot be async.
    async init(window: ServiceWorkerGlobalScope): Promise<void> {
        // If the cache is already initialized, return.
        if (this.initialized != undefined) return;

        console.log("Initializing routing cache using the IndexedDB API.");
        this.initialized = new Promise((resolve, reject) => {
            const request = window.indexedDB.open("routing-cache", 1);
            request.onupgradeneeded = (_event) => {
                const db = request.result;
                const objectStore = db.createObjectStore('routes');
                objectStore.createIndex('siteObjectId', 'siteObjectId', { unique: true });
            };
            request.onsuccess = () => {
                this.cache = request.result;
                resolve(true);
            };
            request.onerror = () => {
                reject(request.error);
            };
        });
    }

    async get(key: string): Promise<Routes | Empty | undefined> {
        await this.initialized; // Make sure the cache is initialized before using it.
        return new Promise((resolve, reject) => {
            const transaction = this.cache.transaction("routes", "readonly");
            const store = transaction.objectStore("routes");
            const request = store.get(key);
            request.onsuccess = () => {
                const value =  request.result;
                if (!value) {
                    console.log("Routing cache miss: ", value);
                    resolve(undefined);
                }
                if (isRoutes(value) || isEmpty(value)) {
                    console.log(`Routing cache hit for ${key}: `, value);
                    resolve(value);
                }
                resolve(undefined);
            };
            request.onerror = () => {
                reject(request.error);
            };
        });
    }

    async set(key: string, value: Routes | Empty): Promise<void> {
        await this.initialized; // Make sure the cache is initialized before using it.
        return new Promise((resolve, reject) => {
            console.log("Setting routing cache. KV Pair: ", key, value)
            const transaction = this.cache.transaction("routes", "readwrite");
            const store = transaction.objectStore("routes");
            const request = store.put(value, key);
            request.onsuccess = () => {
                resolve();
            };
            request.onerror = () => {
                reject(request.error);
            };
        });
    }

    async delete(key: string): Promise<void> {
        await this.initialized; // Make sure the cache is initialized before using it.
        return new Promise((resolve, reject) => {
            const transaction = this.cache.transaction("routes", "readwrite");
            const store = transaction.objectStore("routes");
            const request = store.delete(key);
            request.onsuccess = () => {
                resolve();
            };
            request.onerror = () => {
                reject(request.error);
            };
        });
    }
}
