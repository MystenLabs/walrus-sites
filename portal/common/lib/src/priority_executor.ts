// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

const DEFAULT_DELAY_BETWEEN_RETRIES_MS = 500;
const DEFAULT_LEGACY_RETRIES = 2;
const LEGACY_PRIORITY_INCREMENT = 100;

// --- Types ---

export interface PriorityUrl {
    url: string;
    retries: number;
    priority: number;
}

export type ExecuteResult<T> =
    | { status: "success"; value: T } // Done, return this value
    | { status: "retry-same"; error?: Error } // Retry same URL (if retries left), then next
    | { status: "retry-next"; error?: Error } // Skip to next URL immediately
    | { status: "stop"; error: Error }; // Abort completely, don't try others

// --- Parsing ---

/**
 * Parses a comma-separated list of priority URL entries.
 *
 * New format: URL|RETRIES|METRIC (e.g., "https://rpc.example.com|3|100")
 * Legacy format: plain URLs (e.g., "https://a.com,https://b.com")
 *   - auto-converted with `defaultRetries` and ascending priority (100, 200, ...)
 *
 * @param input - The comma-separated string of URL entries
 * @param defaultRetries - Retries assigned to each URL in legacy format (default: 2)
 * @returns Array of PriorityUrl objects
 * @throws Error if any entry is invalid or formats are mixed
 */
export function parsePriorityUrlList(
    input: string,
    defaultRetries: number = DEFAULT_LEGACY_RETRIES,
): PriorityUrl[] {
    const trimmed = input.trim();
    if (!trimmed) {
        throw new Error("Priority URL list cannot be empty");
    }

    const entries = trimmed.split(",").map((entry) => entry.trim());

    const hasPipe = entries.map((e) => e.includes("|"));
    const allPipe = hasPipe.every(Boolean);
    const nonePipe = hasPipe.every((v) => !v);

    if (!allPipe && !nonePipe) {
        throw new Error(
            "Mixed URL formats detected: some entries use pipe-delimited format (URL|RETRIES|METRIC) " +
                "and some do not. All entries must use the same format.",
        );
    }

    if (nonePipe) {
        // Legacy format: plain URLs
        console.warn(
            "[DEPRECATED] Plain URL list format detected. " +
                "Please migrate to the new format: URL|RETRIES|METRIC " +
                '(e.g., "https://rpc.example.com|3|100").',
        );
        return entries.map((url, index) => {
            try {
                new URL(url);
            } catch {
                throw new Error(`Invalid URL in priority URL entry: "${url}"`);
            }
            return {
                url,
                retries: defaultRetries,
                priority: (index + 1) * LEGACY_PRIORITY_INCREMENT,
            };
        });
    }

    // New pipe-delimited format
    const result: PriorityUrl[] = [];

    for (const entry of entries) {
        const parts = entry.split("|");
        if (parts.length !== 3) {
            throw new Error(
                `Invalid priority URL entry: "${entry}". Expected format: URL|RETRIES|METRIC`,
            );
        }

        const [url, retriesStr, priorityStr] = parts;

        // Validate URL
        try {
            new URL(url);
        } catch {
            throw new Error(`Invalid URL in priority URL entry: "${url}"`);
        }

        // Validate retries
        const retries = parseInt(retriesStr, 10);
        if (isNaN(retries) || retries < 0) {
            throw new Error(
                `Invalid retries value in priority URL entry: "${retriesStr}". Must be a non-negative integer.`,
            );
        }

        // Validate metric
        const priority = parseInt(priorityStr, 10);
        if (isNaN(priority)) {
            throw new Error(
                `Invalid metric value in priority URL entry: "${priorityStr}". Must be an integer.`,
            );
        }

        result.push({ url, retries, priority });
    }

    return result;
}

// --- Executor ---

/**
 * Executes operations against a prioritized list of URLs with retry logic.
 *
 * The executor tries each URL in priority order. For each URL, it will retry
 * up to the configured number of times if the execute function returns 'retry-same'.
 * If 'retry-next' is returned, it immediately moves to the next URL.
 * If 'stop' is returned, it aborts completely with the provided error.
 */
export class PriorityExecutor {
    private readonly sortedItems: readonly PriorityUrl[];
    private readonly delayBetweenRetriesMs: number;

    constructor(
        items: PriorityUrl[],
        delayBetweenRetriesMs: number = DEFAULT_DELAY_BETWEEN_RETRIES_MS,
    ) {
        // Sort by priority (ascending) and freeze to prevent mutation
        this.sortedItems = Object.freeze([...items].sort((a, b) => a.priority - b.priority));
        this.delayBetweenRetriesMs = delayBetweenRetriesMs;
    }

    /**
     * Returns the highest priority URL (first in sorted list).
     */
    getHighestPriorityUrl(): string | undefined {
        return this.sortedItems[0]?.url;
    }

    private createAggregateError(errors: Error[], message: string): AggregateError {
        const summary = errors
            .map((e) => {
                const cause = e.cause instanceof Error ? `: ${e.cause.message}` : "";
                return `${e.message}${cause}`;
            })
            .join("; ");
        return new AggregateError(errors, `${message}: ${summary}`);
    }

    /**
     * Executes the given function against URLs in priority order with retry logic.
     *
     * @param execute - Function that attempts an operation against a URL and returns a result
     * @returns The successful result value
     * @throws AggregateError if all URLs are exhausted, Error if 'stop' is returned
     */
    async invoke<TResult>(
        execute: (url: string) => Promise<ExecuteResult<TResult>>,
    ): Promise<TResult> {
        if (this.sortedItems.length === 0) {
            throw new Error("No URLs available to execute against");
        }

        const errors: Error[] = [];

        urlLoop: for (const item of this.sortedItems) {
            retryLoop: for (let attempt = 0; attempt <= item.retries; attempt++) {
                const result = await execute(item.url);

                switch (result.status) {
                    case "success":
                        return result.value;

                    case "retry-same":
                        errors.push(
                            new Error(`retry-same from ${item.url} (attempt ${attempt + 1})`, {
                                cause: result.error,
                            }),
                        );
                        if (attempt < item.retries) {
                            await new Promise((resolve) =>
                                setTimeout(resolve, this.delayBetweenRetriesMs),
                            );
                            continue retryLoop;
                        }
                        continue urlLoop;

                    case "retry-next":
                        errors.push(
                            new Error(`retry-next from ${item.url}`, { cause: result.error }),
                        );
                        continue urlLoop;

                    case "stop":
                        errors.push(new Error(`stop from ${item.url}`, { cause: result.error }));
                        throw this.createAggregateError(errors, "Stopped");

                    default: {
                        const _exhaustive: never = result;
                        throw new Error(`Unhandled status: ${JSON.stringify(_exhaustive)}`);
                    }
                }
            }
        }

        throw this.createAggregateError(errors, "All URLs exhausted");
    }
}
