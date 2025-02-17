// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

/**
 * Returns the URL to fetch the blob of given ID from the aggregator/cache.
 *
 * @param blob_id - The blob ID to fetch from the aggregator.
 * @param aggregatorUrl - The aggregator URL string.
 */
export function aggregatorEndpoint(blob_id: string, aggregatorUrl: string): URL {
    if (aggregatorUrl.endsWith("/")) {
        throw new Error("Aggregator URL must not end with a slash.");
    }
    return new URL(`${aggregatorUrl}/v1/blobs/${encodeURIComponent(blob_id)}`) as unknown as URL;
}
