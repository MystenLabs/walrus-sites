// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

const DELAY_BETWEEN_RETRIES_MS = 1000;

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
 * Format: URL|RETRIES|PRIORITY (e.g., "https://rpc.example.com|3|100")
 *
 * @param input - The comma-separated string of URL entries
 * @returns Array of PriorityUrl objects sorted by priority (ascending)
 * @throws Error if any entry is invalid
 */
export function parsePriorityUrlList(input: string): PriorityUrl[] {
    const trimmed = input.trim();
    if (!trimmed) {
        throw new Error("Priority URL list cannot be empty");
    }

    const entries = trimmed.split(",").map((entry) => entry.trim());
    const result: PriorityUrl[] = [];

    for (const entry of entries) {
        const parts = entry.split("|");
        if (parts.length !== 3) {
            throw new Error(
                `Invalid priority URL entry: "${entry}". Expected format: URL|RETRIES|PRIORITY`,
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

        // Validate priority
        const priority = parseInt(priorityStr, 10);
        if (isNaN(priority)) {
            throw new Error(
                `Invalid priority value in priority URL entry: "${priorityStr}". Must be an integer.`,
            );
        }

        result.push({ url, retries, priority });
    }

    // Sort by priority (ascending - lower number = higher priority)
    return result.sort((a, b) => a.priority - b.priority);
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

    constructor(items: PriorityUrl[]) {
        // Sort by priority (ascending) and freeze to prevent mutation
        this.sortedItems = Object.freeze([...items].sort((a, b) => a.priority - b.priority));
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
                                setTimeout(resolve, DELAY_BETWEEN_RETRIES_MS),
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
