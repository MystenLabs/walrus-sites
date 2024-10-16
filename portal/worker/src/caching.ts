// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { resolveAndFetchPage } from "@lib/page_fetching";
import { SuiClient, getFullnodeUrl } from "@mysten/sui/client";
import { NETWORK } from "@lib/constants";
import { DomainDetails } from "@lib/types";

const CACHE_NAME = "walrus-sites-cache";
const CACHE_EXPIRATION_TIME = 24 * 60 * 60 * 1000; // 24 hours in milliseconds

/**
 * Respond to the request using the cache API.
 */
export default async function resolveWithCache(
    resolvedObjectId: string,
    parsedUrl: DomainDetails,
    urlString: string,
): Promise<Response> {
    const cache = await caches.open(CACHE_NAME);
    const cachedResponse = await cache.match(urlString);
    const cacheWasFresh = !(await cleanExpiredCache(cachedResponse, urlString));

    let isCacheSameAsNetwork: boolean;
    if (cachedResponse && cacheWasFresh) {
        console.log("Cache hit!");
        try {
            isCacheSameAsNetwork = await checkCachedVersionMatchesOnChain(
                resolvedObjectId,
                cachedResponse,
            );
            if (isCacheSameAsNetwork) return cachedResponse;
        } catch (e) {
            console.error("Error checking cache version against chain:", e);
        }
    }
    console.log("Cache miss!", urlString);
    const resolvedPage = await resolveAndFetchPage(parsedUrl, resolvedObjectId);

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

/**
 * Check if the cached version of the Resource object matches the current on-chain version.
 *
 * @param cachedResponse the response to check the version of
 * @returns true if the cached version matches the current version of the Resource object
 */
async function checkCachedVersionMatchesOnChain(
    resolvedObjectId: string,
    cachedResponse: Response,
): Promise<boolean> {
    if (!cachedResponse) {
        throw new Error("Cached response is null!");
    }
    const rpcUrl = getFullnodeUrl(NETWORK);
    const client = new SuiClient({ url: rpcUrl });
    const cachedVersion = cachedResponse.headers.get("x-resource-sui-object-version");
    const objectId = cachedResponse.headers.get("x-resource-sui-object-id");
    if (!cachedVersion || !objectId) {
        throw new Error("Cached response does not have the required headers");
    }

    if (objectId !== resolvedObjectId) {
        // The object ID has changed, so the cache is invalid.
        return false;
    }

    const resourceObject = await client.getObject({ id: objectId });
    if (!resourceObject.data) {
        throw new Error("Could not retrieve Resource object.");
    }
    console.log("Cached version: ", cachedVersion);
    console.log("Current version: ", resourceObject.data?.version);
    const currentObjectVersion = resourceObject.data?.version;
    return cachedVersion === currentObjectVersion;
}
