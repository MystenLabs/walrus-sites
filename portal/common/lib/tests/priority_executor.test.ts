// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import {
    parsePriorityUrlList,
    PriorityExecutor,
    PriorityUrl,
    ExecuteResult,
} from "@lib/priority_executor";

describe("parsePriorityUrlList", () => {
    it("parses a single entry correctly", () => {
        const result = parsePriorityUrlList("https://example.com|3|100");
        expect(result).toEqual([{ url: "https://example.com", retries: 3, priority: 100 }]);
    });

    it("parses multiple entries and sorts by priority", () => {
        const result = parsePriorityUrlList(
            "https://low.com|1|500,https://high.com|3|100,https://mid.com|2|200",
        );
        expect(result).toEqual([
            { url: "https://high.com", retries: 3, priority: 100 },
            { url: "https://mid.com", retries: 2, priority: 200 },
            { url: "https://low.com", retries: 1, priority: 500 },
        ]);
    });

    it("handles whitespace in entries", () => {
        const result = parsePriorityUrlList(" https://a.com|1|100 , https://b.com|2|200 ");
        expect(result).toHaveLength(2);
        expect(result[0].url).toBe("https://a.com");
        expect(result[1].url).toBe("https://b.com");
    });

    it("throws on empty input", () => {
        expect(() => parsePriorityUrlList("")).toThrow("Priority URL list cannot be empty");
        expect(() => parsePriorityUrlList("   ")).toThrow("Priority URL list cannot be empty");
    });

    it("throws on missing fields", () => {
        expect(() => parsePriorityUrlList("https://example.com|100")).toThrow(
            /Expected format: URL\|RETRIES\|PRIORITY/,
        );
        expect(() => parsePriorityUrlList("https://example.com")).toThrow(
            /Expected format: URL\|RETRIES\|PRIORITY/,
        );
    });

    it("throws on invalid URL", () => {
        expect(() => parsePriorityUrlList("not-a-url|1|100")).toThrow(/Invalid URL/);
    });

    it("throws on invalid retries", () => {
        expect(() => parsePriorityUrlList("https://example.com|abc|100")).toThrow(
            /Invalid retries value/,
        );
        expect(() => parsePriorityUrlList("https://example.com|-1|100")).toThrow(
            /Invalid retries value/,
        );
    });

    it("throws on invalid priority", () => {
        expect(() => parsePriorityUrlList("https://example.com|1|abc")).toThrow(
            /Invalid priority value/,
        );
    });

    it("allows negative priority values", () => {
        const result = parsePriorityUrlList("https://example.com|1|-10");
        expect(result[0].priority).toBe(-10);
    });

    it("allows zero retries", () => {
        const result = parsePriorityUrlList("https://example.com|0|100");
        expect(result[0].retries).toBe(0);
    });
});

describe("PriorityExecutor", () => {
    function createExecutor(items: PriorityUrl[]): PriorityExecutor {
        return new PriorityExecutor(items);
    }

    describe("basic execution", () => {
        it("returns success value on first try", async () => {
            const items: PriorityUrl[] = [{ url: "https://a.com", retries: 2, priority: 100 }];
            const executor = createExecutor(items);

            const result = await executor.invoke(async (url) => ({
                status: "success",
                value: `got ${url}`,
            }));

            expect(result).toBe("got https://a.com");
        });

        it("throws on empty items list", async () => {
            const executor = createExecutor([]);

            await expect(
                executor.invoke(async () => ({ status: "success", value: "ok" })),
            ).rejects.toThrow("No URLs available");
        });
    });

    describe("getHighestPriorityUrl", () => {
        it("returns highest priority URL (lowest priority number)", () => {
            const items: PriorityUrl[] = [
                { url: "https://low.com", retries: 1, priority: 500 },
                { url: "https://high.com", retries: 3, priority: 100 },
                { url: "https://mid.com", retries: 2, priority: 200 },
            ];
            const executor = createExecutor(items);

            expect(executor.getHighestPriorityUrl()).toBe("https://high.com");
        });

        it("returns undefined for empty list", () => {
            const executor = createExecutor([]);
            expect(executor.getHighestPriorityUrl()).toBeUndefined();
        });
    });

    describe("retry-same behavior", () => {
        beforeEach(() => {
            vi.useFakeTimers();
        });

        afterEach(() => {
            vi.useRealTimers();
        });

        it("retries same URL up to retries count", async () => {
            const items: PriorityUrl[] = [{ url: "https://a.com", retries: 2, priority: 100 }];
            const executor = createExecutor(items);
            let attempts = 0;

            const promise = executor.invoke(async (): Promise<ExecuteResult<string>> => {
                attempts++;
                if (attempts < 3) {
                    return { status: "retry-same" };
                }
                return { status: "success", value: "finally" };
            });

            await vi.runAllTimersAsync();
            const result = await promise;

            expect(result).toBe("finally");
            expect(attempts).toBe(3); // 1 initial + 2 retries
        });

        it("moves to next URL when retries exhausted", async () => {
            const items: PriorityUrl[] = [
                { url: "https://a.com", retries: 1, priority: 100 },
                { url: "https://b.com", retries: 0, priority: 200 },
            ];
            const executor = createExecutor(items);
            const urlsAttempted: string[] = [];

            const promise = executor.invoke(async (url): Promise<ExecuteResult<string>> => {
                urlsAttempted.push(url);
                if (url === "https://a.com") {
                    return { status: "retry-same" };
                }
                return { status: "success", value: "from b" };
            });

            await vi.runAllTimersAsync();
            const result = await promise;

            expect(result).toBe("from b");
            expect(urlsAttempted).toEqual([
                "https://a.com",
                "https://a.com", // one retry
                "https://b.com",
            ]);
        });

        it("delays 1 second between retry-same attempts", async () => {
            const items: PriorityUrl[] = [{ url: "https://a.com", retries: 2, priority: 100 }];
            const executor = createExecutor(items);
            const attemptTimes: number[] = [];

            const promise = executor.invoke(async (): Promise<ExecuteResult<string>> => {
                attemptTimes.push(Date.now());
                if (attemptTimes.length < 3) {
                    return { status: "retry-same" };
                }
                return { status: "success", value: "done" };
            });

            // Advance time and let the promise resolve
            await vi.runAllTimersAsync();
            await promise;

            expect(attemptTimes).toHaveLength(3);
            // First attempt at time 0
            // Second attempt after 1000ms delay
            // Third attempt after another 1000ms delay
            expect(attemptTimes[1] - attemptTimes[0]).toBe(1000);
            expect(attemptTimes[2] - attemptTimes[1]).toBe(1000);
        });
    });

    describe("retry-next behavior", () => {
        it("immediately skips to next URL", async () => {
            const items: PriorityUrl[] = [
                { url: "https://a.com", retries: 5, priority: 100 },
                { url: "https://b.com", retries: 0, priority: 200 },
            ];
            const executor = createExecutor(items);
            const urlsAttempted: string[] = [];

            const result = await executor.invoke(async (url): Promise<ExecuteResult<string>> => {
                urlsAttempted.push(url);
                if (url === "https://a.com") {
                    return { status: "retry-next" };
                }
                return { status: "success", value: "from b" };
            });

            expect(result).toBe("from b");
            // Should NOT have retried a.com despite retries: 5
            expect(urlsAttempted).toEqual(["https://a.com", "https://b.com"]);
        });
    });

    describe("stop behavior", () => {
        it("throws error immediately without trying other URLs", async () => {
            const items: PriorityUrl[] = [
                { url: "https://a.com", retries: 2, priority: 100 },
                { url: "https://b.com", retries: 2, priority: 200 },
            ];
            const executor = createExecutor(items);
            const urlsAttempted: string[] = [];

            await expect(
                executor.invoke(async (url): Promise<ExecuteResult<string>> => {
                    urlsAttempted.push(url);
                    return { status: "stop", error: new Error("client error") };
                }),
            ).rejects.toThrow("client error");

            expect(urlsAttempted).toEqual(["https://a.com"]);
        });
    });

    describe("exhaustion behavior", () => {
        beforeEach(() => {
            vi.useFakeTimers();
        });

        afterEach(() => {
            vi.useRealTimers();
        });

        it("throws AggregateError when all URLs exhausted", async () => {
            const items: PriorityUrl[] = [
                { url: "https://a.com", retries: 1, priority: 100 },
                { url: "https://b.com", retries: 1, priority: 200 },
            ];
            const executor = createExecutor(items);

            const promise = executor.invoke(
                async (): Promise<ExecuteResult<string>> => ({
                    status: "retry-same",
                }),
            );

            // Attach catch handler immediately to prevent unhandled rejection warning
            let caughtError: unknown;
            const handledPromise = promise.catch((e) => {
                caughtError = e;
            });

            // Advance timers to handle delays between retries
            await vi.runAllTimersAsync();
            await handledPromise;

            expect(caughtError).toBeInstanceOf(AggregateError);
            expect((caughtError as Error).message).toContain("All URLs exhausted");
            // Message should include details from each error
            expect((caughtError as Error).message).toContain("https://a.com");
            expect((caughtError as Error).message).toContain("https://b.com");
        });

        it("AggregateError contains all retry errors", async () => {
            const items: PriorityUrl[] = [
                { url: "https://a.com", retries: 1, priority: 100 },
                { url: "https://b.com", retries: 0, priority: 200 },
            ];
            const executor = createExecutor(items);

            const promise = executor.invoke(async (url): Promise<ExecuteResult<string>> => {
                if (url === "https://a.com") {
                    return { status: "retry-same" };
                }
                return { status: "retry-next" };
            });

            // Attach catch handler immediately to prevent unhandled rejection warning
            let caughtError: unknown;
            const handledPromise = promise.catch((e) => {
                caughtError = e;
            });

            await vi.runAllTimersAsync();
            await handledPromise;

            expect(caughtError).toBeInstanceOf(AggregateError);
            const aggError = caughtError as AggregateError;
            // a.com: 2 retry-same attempts (1 initial + 1 retry)
            // b.com: 1 retry-next
            expect(aggError.errors).toHaveLength(3);
            expect(aggError.errors[0].message).toContain("https://a.com");
            expect(aggError.errors[1].message).toContain("https://a.com");
            expect(aggError.errors[2].message).toContain("https://b.com");
        });
    });

    describe("priority order", () => {
        it("tries URLs in priority order", async () => {
            const items: PriorityUrl[] = [
                { url: "https://high.com", retries: 0, priority: 100 },
                { url: "https://low.com", retries: 0, priority: 500 },
            ];
            const executor = createExecutor(items);
            const urlsAttempted: string[] = [];

            await executor.invoke(async (url): Promise<ExecuteResult<string>> => {
                urlsAttempted.push(url);
                if (url === "https://high.com") {
                    return { status: "retry-next" };
                }
                return { status: "success", value: "ok" };
            });

            expect(urlsAttempted[0]).toBe("https://high.com");
            expect(urlsAttempted[1]).toBe("https://low.com");
        });
    });

    describe("complex scenarios", () => {
        beforeEach(() => {
            vi.useFakeTimers();
        });

        afterEach(() => {
            vi.useRealTimers();
        });

        it("handles mixed retry behaviors correctly", async () => {
            const items: PriorityUrl[] = [
                { url: "https://a.com", retries: 2, priority: 100 }, // Will retry-same twice
                { url: "https://b.com", retries: 3, priority: 200 }, // Will retry-next
                { url: "https://c.com", retries: 1, priority: 300 }, // Will succeed
            ];
            const executor = createExecutor(items);
            const attempts: { url: string; attempt: number }[] = [];
            const attemptCounts = new Map<string, number>();

            const promise = executor.invoke(async (url): Promise<ExecuteResult<string>> => {
                const count = (attemptCounts.get(url) || 0) + 1;
                attemptCounts.set(url, count);
                attempts.push({ url, attempt: count });

                if (url === "https://a.com") {
                    return { status: "retry-same" }; // Will exhaust retries
                }
                if (url === "https://b.com") {
                    return { status: "retry-next" }; // Will skip immediately
                }
                return { status: "success", value: "from c" };
            });

            await vi.runAllTimersAsync();
            const result = await promise;

            expect(result).toBe("from c");
            expect(attemptCounts.get("https://a.com")).toBe(3); // 1 + 2 retries
            expect(attemptCounts.get("https://b.com")).toBe(1); // Skipped immediately
            expect(attemptCounts.get("https://c.com")).toBe(1); // Succeeded
        });
    });
});
