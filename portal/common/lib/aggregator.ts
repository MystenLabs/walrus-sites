// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

/**
 * Returns the URL to fetch the blob of given ID from the aggregator/cache.
 *
 * @param blob_id - The blob ID to fetch from the aggregator.
 * @param aggregatorUrl - The aggregator URL string.
 */
export function blobAggregatorEndpoint(blob_id: string, aggregatorUrl: string): URL {
	const cleanAggregatorUrl = aggregatorUrl.endsWith("/") ? aggregatorUrl.slice(0, -1) : aggregatorUrl;
	return new URL(`${cleanAggregatorUrl}/v1/blobs/${encodeURIComponent(blob_id)}`) as URL;
}

/**
 * Returns the URL to fetch the blob by quilt patch ID from the aggregator/cache.
 *
 * @param quilt_patch_id - The quilt patch ID to fetch from the aggregator.
 * @param aggregatorUrl - The aggregator URL string.
 */
export function quiltAggregatorEndpoint(quilt_patch_id: string, aggregatorUrl: string): URL {
	const cleanAggregatorUrl = aggregatorUrl.endsWith("/") ? aggregatorUrl.slice(0, -1) : aggregatorUrl;
	return new URL(
		`${cleanAggregatorUrl}/v1/blobs/by-quilt-patch-id/${encodeURIComponent(quilt_patch_id)}`,
	) as URL;
}
