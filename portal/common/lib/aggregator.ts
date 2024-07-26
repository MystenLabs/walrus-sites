// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { AGGREGATOR } from "./constants";

/**
 * Returns the URL to fetch the blob of given ID from the aggregator/cache.
 */
export function aggregatorEndpoint(blob_id: string): URL {
    return new URL(AGGREGATOR + "/v1/" + encodeURIComponent(blob_id));
}
