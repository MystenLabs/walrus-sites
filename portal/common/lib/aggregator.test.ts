// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { describe, expect, test } from "vitest";
import { aggregatorEndpoint } from "./aggregator";

describe("aggregatorEndpoint", () => {
    test("blob_id -> URL", () => {
        const expected = 'https://aggregator-devnet.walrus.space/v1/blob_id';

        const blob_id = "blob_id";
        const url = aggregatorEndpoint(blob_id);
        expect(url.toString()).toEqual(expected);
    });

});
