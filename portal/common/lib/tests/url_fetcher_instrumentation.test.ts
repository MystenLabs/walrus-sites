// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { describe, it, expect, vi, beforeAll, afterAll } from "vitest";
import { UrlFetcher } from "@lib/url_fetcher";
import { ResourceFetcher } from "@lib/resource";
import { SuiNSResolver } from "@lib/suins";
import { WalrusSitesRouter } from "@lib/routing";
import { RPCSelector } from "@lib/rpc_selector";
import { SuiClientTypes } from "@mysten/sui/client";
import { PriorityExecutor, PriorityUrl } from "@lib/priority_executor";
import { instrumentationFacade } from "@lib/instrumentation";
import { ResourceStruct, DynamicFieldStruct, ResourcePathStruct } from "@lib/bcs_data_parsing";
import { sha256 } from "@lib/crypto";
import { createServer } from "node:http";
import type { Server } from "node:http";

describe("UrlFetcher records aggregator timing with mock servers", () => {
    // TESTING STRATEGY:
    // The Walrus Aggregator is mocked at the network boundary with a real HTTP
    // server, so URL construction, timing, and instrumentation run naturally.
    // The Sui resource lookup is mocked at the RPCSelector boundary (the gRPC
    // wire format makes a hand-rolled fullnode impractical), returning the
    // resource's BCS content directly.
    let mockAggregatorServer: Server;
    let aggregatorUrl: string;
    let dynamicFieldContent: Uint8Array;

    const siteObjectId = "0x7a95e4be3948415b852fb287d455166a276d7a52f1a567b4a26b6b5e9c753158";

    beforeAll(async () => {
        const testBlobData = new Uint8Array(8);

        mockAggregatorServer = createServer((_req, res) => {
            res.writeHead(200, { "Content-Type": "application/octet-stream" });
            res.end(testBlobData);
        });
        mockAggregatorServer.listen(0);
        const aggregatorAddress = mockAggregatorServer.address() as { port: number };
        aggregatorUrl = `http://localhost:${aggregatorAddress.port}`;

        // Build the BCS content the dynamic field object would carry on-chain,
        // with a blob_hash matching the bytes the mock aggregator serves.
        const testBlobDataHashBytes = await sha256(testBlobData.buffer as ArrayBuffer);
        let hashBigInt = 0n;
        for (let i = testBlobDataHashBytes.length - 1; i >= 0; i--) {
            hashBigInt = (hashBigInt << 8n) | BigInt(testBlobDataHashBytes[i]);
        }

        const testResource = {
            path: "/test.html",
            headers: new Map(),
            blob_id: "166116566679321753338010777976669723006",
            blob_hash: hashBigInt.toString(),
            range: null,
        };

        dynamicFieldContent = DynamicFieldStruct(ResourcePathStruct, ResourceStruct)
            .serialize({
                parentId: siteObjectId,
                name: { path: "/test.html" },
                value: testResource,
            })
            .toBytes();
    });

    afterAll(() => {
        mockAggregatorServer.close();
    });

    it("should record aggregator time when fetching a resource", async () => {
        const rpcPriorityUrls: PriorityUrl[] = [{ url: "http://unused", retries: 0, metric: 100 }];
        const rpcSelector = new RPCSelector(rpcPriorityUrls, "testnet");
        const wsRouter = new WalrusSitesRouter(rpcSelector);
        const sitePackage = "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";

        // Primary site object (no redirect) + the dynamic field carrying the resource.
        // TODO(tech-debt): partial mocks — cast because SuiClientTypes.Object also
        // requires owner/type/previousTransaction/objectBcs/json, unused by this test.
        vi.spyOn(rpcSelector, "multiGetObjects").mockResolvedValue([
            { objectId: siteObjectId, version: "1", digest: "primary", display: null },
            { objectId: "0xdf", version: "1", digest: "df", content: dynamicFieldContent },
        ] as unknown as SuiClientTypes.Object<{ content: true; display: true }>[]);

        const aggregatorPriorityUrls: PriorityUrl[] = [
            { url: aggregatorUrl, retries: 2, metric: 100 },
        ];
        const aggregatorExecutor = new PriorityExecutor(aggregatorPriorityUrls);

        const urlFetcher = new UrlFetcher(
            new ResourceFetcher(rpcSelector, sitePackage),
            new SuiNSResolver(rpcSelector),
            wsRouter,
            aggregatorExecutor,
            true,
        );

        const recordTimeSpy = vi.spyOn(instrumentationFacade, "recordAggregatorTime");

        await urlFetcher.fetchUrl(siteObjectId, "/test.html");

        // Verify recordAggregatorTime was called
        expect(recordTimeSpy).toHaveBeenCalledTimes(1);

        // Verify call parameters
        const [duration, siteId] = recordTimeSpy.mock.calls[0];

        // Duration should be a non-negative number
        expect(duration).toBeTypeOf("number");
        expect(duration).toBeGreaterThanOrEqual(0);

        // Should pass siteId directly (not wrapped in an object).
        expect(siteId).toBe(siteObjectId);
    });
});
