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
 * Returns true if the error means the fullnode cleanly responded that an object
 * (e.g. an unregistered SuiNS name's dynamic field) does not exist. This is an
 * authoritative answer, so retrying won't change it.
 *
 * The gRPC core API throws a plain Error like "Object 0x… not found" with no
 * structured code. We also keep the legacy JSON-RPC `code: "notExists"` check
 * for safety. Detection is duck-typed because no error class is exported.
 */
function isNotExistsError(error: unknown): boolean {
    if (typeof error !== "object" || error === null) {
        return false;
    }
    const { code, message } = error as { code?: unknown; message?: unknown };
    if (code === "notExists") {
        return true;
    }
    return typeof message === "string" && /not found|does not exist/i.test(message);
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
                // ObjectError(notExists) means the FN authoritatively answered
                // "this object does not exist" — retrying won't change the
                // answer.
                if (isNotExistsError(error)) {
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
        try {
            return await this.invokeWithFailover((client) => client.getNameRecord(name));
        } catch (error) {
            // The executor wraps the ObjectError in: AggregateError → Error
            // ("stop from <url>", { cause: ObjectError }). Check the cause
            // chain for the notExists code.
            if (
                error instanceof AggregateError &&
                error.errors.some((e) => isNotExistsError((e as Error & { cause?: unknown }).cause))
            ) {
                logger.info("SuiNS name not registered (FN responded notExists)", { name });
                return null;
            }
            throw error;
        }
    }
}
