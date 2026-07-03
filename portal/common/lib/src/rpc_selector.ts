// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { SuiClientTypes } from "@mysten/sui/client";
import { SuiGrpcClient } from "@mysten/sui/grpc";
import { SuinsClient } from "@mysten/suins";
import logger, { formatError } from "@lib/logger";
import { NameRecord, Network } from "@lib/types";
import { ExecuteResult, PriorityExecutor, PriorityUrl } from "@lib/priority_executor";

const DEFAULT_RPC_REQUEST_TIMEOUT_MS = 7_000;
export const rpcRequestTimeoutMs =
    Number(process.env.RPC_REQUEST_TIMEOUT_MS) || DEFAULT_RPC_REQUEST_TIMEOUT_MS;

// TODO(tech-debt): leftover ceremony — it declares only multiGetObjects (not
// even getNameRecord), isn't exported, and RPCSelector is its single
// implementer. Nothing types against it; it can be deleted.
interface RPCSelectorInterface {
    multiGetObjects<Include extends SuiClientTypes.ObjectInclude>(
        objectIds: string[],
        include: Include,
    ): Promise<SuiClientTypes.GetObjectsResponse<Include>["objects"]>;
}

class WrappedSuiClient extends SuiGrpcClient {
    private url: string;
    private suinsClient: SuinsClient;

    constructor(url: string, network: Network) {
        super({ baseUrl: url, network });
        this.url = url;
        this.suinsClient = new SuinsClient({
            client: this,
            network,
        });
    }

    public getURL(): string {
        return this.url;
    }

    // Extends the SuiClient class to add a method to get a SuiNS record.
    // Useful for treating the SuiClient as a SuiNS client during the invokeWithFailover method.
    public async getNameRecord(name: string): Promise<NameRecord | null> {
        return await this.suinsClient.getNameRecord(name);
    }
}

/**
 * True only for the authoritative "object doesn't exist" miss. A transient
 * HTTP 404 (`RpcError`) also says "not found" but carries a gRPC `code`, so
 * we key off shape: a plain `Error`, no code, message `"Object 0x… not found"`
 * (pinned by the drift-guard tests).
 */
export function isObjectNotFoundError(error: unknown): boolean {
    if (!(error instanceof Error)) {
        return false;
    }
    // Transport errors (RpcError) always carry a gRPC status `code` string; a
    // genuine missing-object error has none.
    if (typeof (error as { code?: unknown }).code === "string") {
        return false;
    }
    return /^Object 0x[0-9a-f]+ not found/i.test(error.message);
}

/**
 * True only for the authoritative "SuiNS name not registered" error, which the
 * caller resolves to `null`; every other error stays retryable.
 *
 * TODO(tech-debt): only needed because `@mysten/suins` getNameRecord throws on
 * a miss instead of returning null as its signature promises; delete once
 * fixed upstream.
 */
export function isNameNotRegisteredError(error: unknown): boolean {
    if (isObjectNotFoundError(error)) {
        return true;
    }
    if (!(error instanceof Error)) {
        return false;
    }
    if (typeof (error as { code?: unknown }).code === "string") {
        return false;
    }
    return /not registered/i.test(error.message);
}

export class RPCSelector implements RPCSelectorInterface {
    private executor: PriorityExecutor;
    private clients: Map<string, WrappedSuiClient>;
    private readonly timeoutMs: number = rpcRequestTimeoutMs;

    constructor(priorityUrls: PriorityUrl[], network: Network) {
        this.executor = new PriorityExecutor(priorityUrls);
        this.clients = new Map(
            priorityUrls.map((p) => [p.url, new WrappedSuiClient(p.url, network)]),
        );
    }

    // General method to call clients in priority order with failover.
    private async invokeWithFailover<T>(fn: (client: WrappedSuiClient) => Promise<T>): Promise<T> {
        if (this.clients.size === 0) {
            throw new Error("No available clients to handle the request.");
        }

        return this.executor.invoke(async (url): Promise<ExecuteResult<T>> => {
            const client = this.clients.get(url);
            if (!client) {
                return {
                    status: "stop",
                    error: new Error(`No client found for URL: ${url}`),
                };
            }

            try {
                // TODO: Move timeout logic to WrappedSuiClient constructor by passing a
                // custom fetch with AbortSignal.timeout() to SuiHTTPTransport. This would
                // simplify invokeWithFailover to just handle retry logic.
                // NOTE: As of early 2026, Bun's AbortSignal stops processing responses but
                // requests still complete in the background. This may change in future versions.
                let timer: ReturnType<typeof setTimeout>;
                const timeoutPromise = new Promise<never>((_, reject) => {
                    timer = setTimeout(
                        () => reject(new Error("Request timed out")),
                        this.timeoutMs,
                    );
                });

                try {
                    const result = await Promise.race([fn(client), timeoutPromise]);
                    return { status: "success", value: result };
                } finally {
                    clearTimeout(timer!);
                }
            } catch (error) {
                const wrappedError = error instanceof Error ? error : new Error(String(error));
                // INVALID_ARGUMENT is deterministic: a malformed request gets
                // the same answer from every node, so retrying or failing over
                // only burns the whole retry budget (~6 attempts × 7s).
                if ((error as { code?: unknown }).code === "INVALID_ARGUMENT") {
                    logger.error("RPC call failed with non-retryable error", {
                        url,
                        error: formatError(error),
                    });
                    return { status: "stop", error: wrappedError };
                }
                logger.warn("RPC call failed", { url, error: formatError(error) });
                return { status: "retry-same", error: wrappedError };
            }
        });
    }

    /**
     * Fetches multiple objects by ID, in the same order they were requested.
     *
     * Each entry is the fetched object, or an `Error` when the fullnode reported
     * that object as not existing. The gRPC core API already returns a per-object
     * miss as an `Error` element (not a thrown error), so we hand its result back
     * unchanged and let each caller decide what a miss means — a 404 for a
     * resource, or simply an absent optional field for routes/redirects.
     */
    public async multiGetObjects<Include extends SuiClientTypes.ObjectInclude>(
        objectIds: string[],
        include: Include,
    ): Promise<SuiClientTypes.GetObjectsResponse<Include>["objects"]> {
        logger.info("RPCSelector: multiGetObjects", { objectIds, include });
        const { objects } = await this.invokeWithFailover((client) =>
            client.core.getObjects({ objectIds, include }),
        );
        return objects;
    }

    public async getNameRecord(name: string): Promise<NameRecord | null> {
        logger.info("RPCSelector: getNameRecord", { name });
        return await this.invokeWithFailover(async (client) => {
            try {
                return await client.getNameRecord(name);
            } catch (error) {
                // A name with no on-chain record is an authoritative answer, not
                // a failure. Resolve it to null here: a successful null ends the
                // failover loop, so we stop instead of retrying every node. Any
                // other error propagates and is retried / failed over as usual.
                if (isNameNotRegisteredError(error)) {
                    logger.info("SuiNS name not registered", { name });
                    return null;
                }
                throw error;
            }
        });
    }
}
