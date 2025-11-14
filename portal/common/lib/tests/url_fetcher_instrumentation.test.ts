// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { describe, it, expect, vi, beforeAll, afterAll } from 'vitest';
import { UrlFetcher } from '@lib/url_fetcher';
import { ResourceFetcher } from '@lib/resource';
import { SuiNSResolver } from '@lib/suins';
import { WalrusSitesRouter } from '@lib/routing';
import { RPCSelector } from '@lib/rpc_selector';
import { instrumentationFacade } from '@lib/instrumentation';
import { ResourceStruct, DynamicFieldStruct, ResourcePathStruct } from '@lib/bcs_data_parsing';
import { toBase64 } from '@mysten/sui/utils';
import type { Server } from 'bun';

describe('UrlFetcher records aggregator timing with mock servers', () => {
    // TESTING STRATEGY: Mock at the network boundary
    // We create real HTTP servers that simulate Sui Full Node and Walrus Aggregator.
    // This allows all internal logic (URL construction, timing, instrumentation calls)
    // to run naturally with real HTTP requests, while external dependencies are mocked.
    let mockSuiServer: Server;
    let mockAggregatorServer: Server;
    let suiRpcUrl: string;
    let aggregatorUrl: string;

    beforeAll(() => {
        const testBlobData = new Uint8Array(8);
        mockSuiServer = createMockSuiFullnode(testBlobData);
        mockAggregatorServer = Bun.serve({
            port: 0,
            fetch() {
                return new Response(testBlobData, {
                    status: 200,
                    headers: { "Content-Type": "application/octet-stream" }
                });
            }
        });

        suiRpcUrl = `http://localhost:${mockSuiServer.port}`;
        aggregatorUrl = `http://localhost:${mockAggregatorServer.port}`;
    });

    afterAll(() => {
        mockSuiServer.stop();
        mockAggregatorServer.stop();
    });

    it('should record aggregator time when fetching a resource', async () => {
        const rpcSelector = new RPCSelector([suiRpcUrl], 'testnet');
        const wsRouter = new WalrusSitesRouter(rpcSelector);
        const sitePackage = '0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef';

        const urlFetcher = new UrlFetcher(
            new ResourceFetcher(rpcSelector, sitePackage),
            new SuiNSResolver(rpcSelector),
            wsRouter,
            aggregatorUrl,
            true
        );

        const siteObjectId = '0x7a95e4be3948415b852fb287d455166a276d7a52f1a567b4a26b6b5e9c753158';
        const path = '/test.html';

        const recordTimeSpy = vi.spyOn(instrumentationFacade, 'recordAggregatorTime');

        await urlFetcher.fetchUrl(siteObjectId, path);

        // Verify recordAggregatorTime was called
        expect(recordTimeSpy).toHaveBeenCalledTimes(1);

        // Verify call parameters
        const [duration, metadata] = recordTimeSpy.mock.calls[0];

        // Duration should be a non-negative number
        expect(duration).toBeTypeOf('number');
        expect(duration).toBeGreaterThanOrEqual(0);

        // Metadata should have correct structure
        expect(metadata.siteId).toBe(siteObjectId);
        expect(metadata.path).toBe(path);
        expect(metadata.blobOrPatchId).toBeTypeOf('string');
    });
});

// Helper: Create mock Sui Full Node server
function createMockSuiFullnode(testBlobData: Uint8Array): Server {
    const testBlobDataHashBytes = Bun.SHA256.hash(testBlobData);

    let hashBigInt = 0n;
    for (let i = testBlobDataHashBytes.length - 1; i >= 0; i--) {
        hashBigInt = (hashBigInt << 8n) | BigInt(testBlobDataHashBytes[i]);
    }

    const testResource = {
        path: "/test.html",
        headers: new Map(),
        blob_id: "166116566679321753338010777976669723006",
        blob_hash: hashBigInt.toString(),
        range: null
    };

    const dynamicField = DynamicFieldStruct(
        ResourcePathStruct,
        ResourceStruct
    ).serialize({
        parentId: "0x7a95e4be3948415b852fb287d455166a276d7a52f1a567b4a26b6b5e9c753158",
        name: { path: "/test.html" },
        value: testResource
    });

    const bcsBytes = toBase64(dynamicField.toBytes());

    return Bun.serve({
        port: 0,
        async fetch(req) {
            const body = await req.json() as any;
            const ids = body.params[0] || [];

            return new Response(JSON.stringify({
                jsonrpc: "2.0",
                id: body.id,
                result: [
                    {
                        data: {
                            objectId: ids[0],
                            version: "1",
                            digest: "PrimaryObjectDigest",
                            display: {
                                data: null,
                                error: null
                            }
                        }
                    },
                    {
                        data: {
                            objectId: ids[1],
                            version: "1",
                            digest: "DynamicFieldDigest",
                            bcs: {
                                dataType: "moveObject",
                                bcsBytes,
                                hasPublicTransfer: false,
                                type: "0x2::dynamic_field::Field<0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef::site::ResourcePath, 0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef::resource::Resource>",
                                version: "1"
                            }
                        }
                    }
                ]
            }), {
                headers: { "Content-Type": "application/json" }
            });
        }
    });
}
