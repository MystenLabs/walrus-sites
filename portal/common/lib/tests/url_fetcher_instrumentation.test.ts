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
import { sha256 } from '@lib/crypto';
import { createServer } from 'node:http';
import type { Server } from 'node:http';

describe('UrlFetcher records aggregator timing with mock servers', () => {
    // TESTING STRATEGY: Mock at the network boundary
    // We create real HTTP servers that simulate Sui Full Node and Walrus Aggregator.
    // This allows all internal logic (URL construction, timing, instrumentation calls)
    // to run naturally with real HTTP requests, while external dependencies are mocked.
    let mockSuiServer: Server;
    let mockAggregatorServer: Server;
    let suiRpcUrl: string;
    let aggregatorUrl: string;

    beforeAll(async () => {
        const testBlobData = new Uint8Array(8);
        mockSuiServer = await createMockSuiFullnode(testBlobData);

        mockAggregatorServer = createServer((req, res) => {
            res.writeHead(200, { "Content-Type": "application/octet-stream" });
            res.end(testBlobData);
        });
        mockAggregatorServer.listen(0);

        const suiAddress = mockSuiServer.address() as { port: number };
        const aggregatorAddress = mockAggregatorServer.address() as { port: number };

        suiRpcUrl = `http://localhost:${suiAddress.port}`;
        aggregatorUrl = `http://localhost:${aggregatorAddress.port}`;
    });

    afterAll(() => {
        mockSuiServer.close();
        mockAggregatorServer.close();
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
async function createMockSuiFullnode(testBlobData: Uint8Array): Promise<Server> {
    const testBlobDataHashBytes = await sha256(testBlobData.buffer);

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

    const server = createServer(async (req, res) => {
        let body = '';
        req.on('data', chunk => {
            body += chunk.toString();
        });
        req.on('end', () => {
            const parsedBody = JSON.parse(body);
            const ids = parsedBody.params[0] || [];

            const response = JSON.stringify({
                jsonrpc: "2.0",
                id: parsedBody.id,
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
            });

            res.writeHead(200, { "Content-Type": "application/json" });
            res.end(response);
        });
    });

    server.listen(0);
    return server;
}
