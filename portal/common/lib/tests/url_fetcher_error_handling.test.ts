// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { describe, it, expect, vi, beforeAll, afterAll, beforeEach } from 'vitest';
import { UrlFetcher } from '@lib/url_fetcher';
import { ResourceFetcher } from '@lib/resource';
import { SuiNSResolver } from '@lib/suins';
import { WalrusSitesRouter } from '@lib/routing';
import { HttpStatusCodes } from '@lib/http/http_status_codes';
import { createServer } from 'node:http';
import type { Server } from 'node:http';

// Mock instrumentation to avoid port conflicts with instrumentation.test.ts
vi.mock('@lib/instrumentation', () => ({
    instrumentationFacade: {
        recordAggregatorTime: vi.fn(),
        bumpAggregatorFailRequests: vi.fn(),
        increaseRequestsMade: vi.fn(),
    }
}));

describe('fetchWithRetry error handling', () => {
    let mockAggregatorServer: Server;
    let aggregatorUrl: string;
    let responseStatus: number;
    let requestCount: number;

    // Mock dependencies
    const mockResourceFetcher = {
        fetchResource: vi.fn(),
    } as unknown as ResourceFetcher;

    const mockSuiNSResolver = {} as SuiNSResolver;
    const mockWsRouter = {} as WalrusSitesRouter;

    beforeAll(() => {
        mockAggregatorServer = createServer((req, res) => {
            requestCount++;
            res.writeHead(responseStatus, { "Content-Type": "application/json" });
            if (responseStatus === 404) {
                res.end(JSON.stringify({
                    error: {
                        status: "NOT_FOUND",
                        code: 404,
                        message: "the requested blob ID does not exist on Walrus, ensure that it was entered correctly"
                    }
                }));
            } else if (responseStatus >= 500) {
                res.end(JSON.stringify({
                    error: {
                        status: "INTERNAL_SERVER_ERROR",
                        code: responseStatus,
                        message: "an internal server error occurred"
                    }
                }));
            } else {
                res.end(JSON.stringify({
                    error: {
                        status: "ERROR",
                        code: responseStatus,
                        message: "an error occurred"
                    }
                }));
            }
        });
        mockAggregatorServer.listen(0);
        const addr = mockAggregatorServer.address() as { port: number };
        aggregatorUrl = `http://localhost:${addr.port}`;
    });

    afterAll(() => {
        mockAggregatorServer.close();
    });

    beforeEach(() => {
        requestCount = 0;
    });

    function createUrlFetcher() {
        return new UrlFetcher(
            mockResourceFetcher,
            mockSuiNSResolver,
            mockWsRouter,
            aggregatorUrl,
            true
        );
    }

    function mockValidResource() {
        mockResourceFetcher.fetchResource = vi.fn().mockResolvedValue({
            path: '/test.html',
            headers: new Map(),
            blob_id: '123',
            blob_hash: 'somehash',
            range: null,
            objectId: '0x1',
            version: '1',
        });
    }

    describe('aggregator 404 responses', () => {
        it('should return resourceNotFound and not retry on 404', async () => {
            responseStatus = 404;
            mockValidResource();

            const urlFetcher = createUrlFetcher();
            const response = await urlFetcher.fetchUrl('0x1', '/test.html');

            expect(response.status).toBe(HttpStatusCodes.NOT_FOUND);
            expect(requestCount).toBe(1);
        });
    });

    describe('aggregator 5xx responses', () => {
        it('should return aggregatorFail (503) and retry on 500', async () => {
            responseStatus = 500;
            mockValidResource();

            const urlFetcher = createUrlFetcher();
            const response = await urlFetcher.fetchUrl('0x1', '/test.html');

            expect(response.status).toBe(HttpStatusCodes.SERVICE_UNAVAILABLE);
            expect(requestCount).toBe(3); // 1 initial + 2 retries
        });

        it('should return aggregatorFail (503) on 502', async () => {
            responseStatus = 502;
            mockValidResource();

            const urlFetcher = createUrlFetcher();
            const response = await urlFetcher.fetchUrl('0x1', '/test.html');

            expect(response.status).toBe(HttpStatusCodes.SERVICE_UNAVAILABLE);
        });
    });

    describe('aggregator 4xx responses (non-404)', () => {
        it('should throw error and not retry on 400', async () => {
            responseStatus = 400;
            mockValidResource();

            const urlFetcher = createUrlFetcher();

            await expect(urlFetcher.fetchUrl('0x1', '/test.html')).rejects.toThrow(
                /Unhandled response status from aggregator/
            );
            expect(requestCount).toBe(1);
        });

        it('should throw error and not retry on 403', async () => {
            responseStatus = 403;
            mockValidResource();

            const urlFetcher = createUrlFetcher();

            await expect(urlFetcher.fetchUrl('0x1', '/test.html')).rejects.toThrow(
                /Unhandled response status from aggregator/
            );
            expect(requestCount).toBe(1);
        });
    });
});
