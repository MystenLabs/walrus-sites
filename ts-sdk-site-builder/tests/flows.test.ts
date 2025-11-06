// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { publish_site_flow } from "../src/flows";
import { describe, it, expect } from "bun:test";
import { SiteBuilder } from "../src/site-builder";
import { Transaction } from "@mysten/sui/transactions";
import { join } from "path";

describe("site publishing", () => {
	it("should publish a test site", async () => {
		const configPath = join(process.cwd(), "..", "sites-config.yaml");
		const siteBuilder = new SiteBuilder(configPath);
		const tx = new Transaction();
		// FIXME: expecting to be undefined for a happy path is weird. .
		const tx2 = publish_site_flow(tx, siteBuilder);
        expect(await siteBuilder.run(tx2)).toBeUndefined();
    });
});
