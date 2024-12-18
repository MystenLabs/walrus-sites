// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { urlFetcher } from "./walrus-sites-sw";
import { DomainDetails } from "@lib/types";

const CACHE_NAME = "walrus-sites-cache";
const CACHE_EXPIRATION_TIME = 60 * 1000; // 1 minute in milliseconds

/**
 * Respond to the request using the cache API.
 */
export default async function resolveWithCache(
    parsedUrl: DomainDetails,
    urlString: string,
): Promise<Response> {
    const cache = await caches.open(CACHE_NAME);
    const cachedResponse = await cache.match(urlString);
    const cacheWasFresh = !(await cleanExpiredCache(cachedResponse, urlString));
    if (cachedResponse && cacheWasFresh) {
        console.log("Cache hit and fresh!")
        return cachedResponse;
    }
    console.log("Cache miss!", urlString);
    const resolvedPage = await urlFetcher.resolveDomainAndFetchUrl(parsedUrl, null);
    await tryCachePut(cache, urlString, resolvedPage);

    return resolvedPage;
}

async function tryCachePut(cache: Cache, urlString: string, resolvedPage: Response) {
    try {
        await cache.put(urlString, resolvedPage.clone());
    } catch (e) {
        if (e.name === "QuotaExceededError") {
            const keys = await cache.keys();
            if (keys.length === 0) {
                const breakRecursionError = new Error(
                    "Cache quota exceeded, and there are no entries to delete.",
                    {
                        cause: "HumongousEntriesError",
                    },
                );
                throw breakRecursionError;
            }
            console.warn("Cache quota exceeded. Deleting older entries...");
            // Delete at most N oldest entries if available.
            for (let i = 0; i < 50; i++) {
                if (i > keys.length) break;
                const oldestKey = keys[i];
                await cache.delete(oldestKey);
                console.log("Deleted cache entry:", oldestKey);
            }
            // Retrying...
            await tryCachePut(cache, urlString, resolvedPage);
        } else {
            // If not a QuotaExceededError, log the error.
            // No need to rethrow, as the error is not critical.
            // It's just about caching.
            console.error("Error caching the response:", e);
        }
    }
}

/**
 * Removes an entry of the cache, if that entry is expired.
 *
 * The expiration time is set by the `CACHE_EXPIRATION_TIME` constant.
 * If the cached response is older than the expiration time, it's no longer
 * "fresh" and it is removed from the cache.
 *
 * @param urlString the key of the cached entry to check
 * @returns true if the cache entry was removed, false otherwise
 */
async function cleanExpiredCache(cachedResponse: Response, urlString: string): Promise<boolean> {
    const cache = await caches.open(CACHE_NAME);
    const now = Date.now();

    if (cachedResponse) {
        // Cache hit!
        const timestamp = parseInt(cachedResponse.headers.get("x-unix-time-cached") || "0");
        const hasExpired = now - timestamp > CACHE_EXPIRATION_TIME;
        if (hasExpired) {
            await cache.delete(urlString);
            console.log("Removed expired cache entry:", urlString);
            return true;
        }
        console.log("Cache entry is still fresh:", urlString);
    }
    return false;
}
