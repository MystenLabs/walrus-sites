// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { SuiClient } from "@mysten/sui/client";
import { publish_site_flow } from "../src/flows";
import { describe, it, expect } from "bun:test";

describe("site publishing", () => {
    it("should publish a test site", async () => {
        const suiClient = new SuiClient({
            network: "testnet",
            url: "https://fullnode.testnet.sui.io",
        });
        const errors = await publish_site_flow(suiClient);
        expect(errors).toBeUndefined()
    });
});
