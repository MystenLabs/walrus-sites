// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { describe, it, expect, vi, beforeAll, afterAll, beforeEach } from "vitest";
import { UrlFetcher } from "@lib/url_fetcher";
import { ResourceFetcher } from "@lib/resource";
import { SuiNSResolver } from "@lib/suins";
import { WalrusSitesRouter } from "@lib/routing";
import { HttpStatusCodes } from "@lib/http/http_status_codes";
import { PriorityExecutor, PriorityUrl } from "@lib/priority_executor";
import { createServer } from "node:http";
import type { Server } from "node:http";

// Mock instrumentation to avoid port conflicts with instrumentation.test.ts
vi.mock("@lib/instrumentation", () => ({
    instrumentationFacade: {
        recordAggregatorTime: vi.fn(),
        bumpAggregatorFailRequests: vi.fn(),
        bumpBlobUnavailableRequests: vi.fn(),
        increaseRequestsMade: vi.fn(),
    },
}));

describe("aggregator error handling with priority executor", () => {
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
                res.end(
                    JSON.stringify({
                        error: {
                            status: "NOT_FOUND",
                            code: 404,
                            message:
                                "the requested blob ID does not exist on Walrus, ensure that it was entered correctly",
                        },
                    }),
                );
            } else if (responseStatus >= 500) {
                res.end(
                    JSON.stringify({
                        error: {
                            status: "INTERNAL_SERVER_ERROR",
                            code: responseStatus,
                            message: "an internal server error occurred",
                        },
                    }),
                );
            } else {
                res.end(
                    JSON.stringify({
                        error: {
                            status: "ERROR",
                            code: responseStatus,
                            message: "an error occurred",
                        },
                    }),
                );
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

    function createUrlFetcher(retries: number = 2) {
        const priorityUrls: PriorityUrl[] = [{ url: aggregatorUrl, retries, metric: 100 }];
        const aggregatorExecutor = new PriorityExecutor(priorityUrls);
        return new UrlFetcher(
            mockResourceFetcher,
            mockSuiNSResolver,
            mockWsRouter,
            aggregatorExecutor,
            true,
        );
    }

    function mockValidResource() {
        mockResourceFetcher.fetchResource = vi.fn().mockResolvedValue({
            path: "/test.html",
            headers: new Map(),
            blob_id: "123",
            blob_hash: "somehash",
            range: null,
            objectId: "0x1",
            version: "1",
        });
    }

    describe("aggregator 404 responses", () => {
        it("should return blobUnavailable error and not retry on 404", async () => {
            responseStatus = 404;
            mockValidResource();

            const urlFetcher = createUrlFetcher();
            const result = await urlFetcher.fetchUrl("0x1", "/test.html");

            expect(result.status).toBe("BlobUnavailable");
            expect(requestCount).toBe(1);

            // Response should contain blob unavailable message with blob ID
            if (result.status === "BlobUnavailable") {
                expect(result.response.status).toBe(HttpStatusCodes.NOT_FOUND);
                const text = await result.response.text();
                expect(text).toContain("no longer available");
                expect(text).toContain("123"); // blob_id from mockValidResource
            }
        });
    });

    describe("aggregator 5xx responses", () => {
        it("should return aggregatorFail error and retry on 500", async () => {
            responseStatus = 500;
            mockValidResource();

            const urlFetcher = createUrlFetcher(2); // 2 retries = 3 total attempts
            const result = await urlFetcher.fetchUrl("0x1", "/test.html");

            expect(result.status).toBe("AggregatorFail");
            if (result.status === "AggregatorFail") {
                expect(result.response.status).toBe(HttpStatusCodes.SERVICE_UNAVAILABLE);
            }
            expect(requestCount).toBe(3); // 1 initial + 2 retries
        });

        it("should return aggregatorFail on 502 without retrying same URL", async () => {
            responseStatus = 502;
            mockValidResource();

            const urlFetcher = createUrlFetcher(5); // Many retries, but 502 triggers retry-next
            const result = await urlFetcher.fetchUrl("0x1", "/test.html");

            expect(result.status).toBe("AggregatorFail");
            if (result.status === "AggregatorFail") {
                expect(result.response.status).toBe(HttpStatusCodes.SERVICE_UNAVAILABLE);
            }
            // Should only try once since 502 triggers retry-next
            expect(requestCount).toBe(1);
        });
    });

    describe("aggregator 4xx responses (non-404)", () => {
        it("should return aggregatorFail and not retry on 400", async () => {
            responseStatus = 400;
            mockValidResource();

            const urlFetcher = createUrlFetcher();
            const result = await urlFetcher.fetchUrl("0x1", "/test.html");

            expect(result.status).toBe("AggregatorFail");
            if (result.status === "AggregatorFail") {
                expect(result.response.status).toBe(HttpStatusCodes.SERVICE_UNAVAILABLE);
            }
            expect(requestCount).toBe(1);
        });

        it("should return aggregatorFail error and not retry on 403", async () => {
            // 403 from aggregator means blob size exceeds configured max - this is
            // an aggregator configuration issue, so we return 503 Service Unavailable
            responseStatus = 403;
            mockValidResource();

            const urlFetcher = createUrlFetcher();
            const result = await urlFetcher.fetchUrl("0x1", "/test.html");

            expect(result.status).toBe("AggregatorFail");
            expect(requestCount).toBe(1);

            // Response should contain aggregator fail message
            if (result.status === "AggregatorFail") {
                expect(result.response.status).toBe(HttpStatusCodes.SERVICE_UNAVAILABLE);
                const text = await result.response.text();
                expect(text).toContain("Failed to contact the aggregator");
            }
        });
    });

    describe("multi-aggregator fallback", () => {
        let mockServer2: Server;
        let aggregatorUrl2: string;
        let server2RequestCount: number;

        beforeAll(() => {
            mockServer2 = createServer((req, res) => {
                server2RequestCount++;
                // Second server always returns success
                res.writeHead(200, { "Content-Type": "application/octet-stream" });
                res.end("test content");
            });
            mockServer2.listen(0);
            const addr = mockServer2.address() as { port: number };
            aggregatorUrl2 = `http://localhost:${addr.port}`;
        });

        afterAll(() => {
            mockServer2.close();
        });

        beforeEach(() => {
            server2RequestCount = 0;
        });

        it("should fall back to second aggregator when first returns 502", async () => {
            responseStatus = 502; // First server returns 502
            mockResourceFetcher.fetchResource = vi.fn().mockResolvedValue({
                path: "/test.html",
                headers: new Map(),
                blob_id: "123",
                blob_hash: "auinVVUgn9bEQVfArtgBbnY/9DWhnPGG92hjFAFD/3I=", // hash of "test content"
                range: null,
                objectId: "0x1",
                version: "1",
            });

            const priorityUrls: PriorityUrl[] = [
                { url: aggregatorUrl, retries: 2, metric: 100 },
                { url: aggregatorUrl2, retries: 2, metric: 200 },
            ];
            const aggregatorExecutor = new PriorityExecutor(priorityUrls);
            const urlFetcher = new UrlFetcher(
                mockResourceFetcher,
                mockSuiNSResolver,
                mockWsRouter,
                aggregatorExecutor,
                true,
            );

            const result = await urlFetcher.fetchUrl("0x1", "/test.html");

            expect(result.status).toBe("Ok");
            if (result.status === "Ok") {
                expect(result.response.status).toBe(200);
            }
            expect(requestCount).toBe(1); // First server tried once
            expect(server2RequestCount).toBe(1); // Second server succeeded
        });
    });
});
