// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import {
    GetDynamicFieldObjectParams,
    GetObjectParams,
    MultiGetObjectsParams,
    SuiClient,
    SuiObjectResponse,
} from "@mysten/sui/client";
import { SuinsClient } from "@mysten/suins";
import logger, { formatError } from "@lib/logger";
import { NameRecord, Network } from "@lib/types";
import { ExecuteResult, PriorityExecutor, PriorityUrl } from "@lib/priority_executor";

interface RPCSelectorInterface {
    getObject(input: GetObjectParams): Promise<SuiObjectResponse>;
    multiGetObjects(input: MultiGetObjectsParams): Promise<SuiObjectResponse[]>;
    getDynamicFieldObject(input: GetDynamicFieldObjectParams): Promise<SuiObjectResponse>;
}

class WrappedSuiClient extends SuiClient {
    private url: string;
    private suinsClient: SuinsClient;

    constructor(url: string, network: Network) {
        super({ url });
        this.url = url;
        this.suinsClient = new SuinsClient({
            client: this as SuiClient,
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

export class RPCSelector implements RPCSelectorInterface {
    private executor: PriorityExecutor;
    private clients: Map<string, WrappedSuiClient>;
    private readonly timeoutMs: number;

    constructor(priorityUrls: PriorityUrl[], network: Network) {
        this.executor = new PriorityExecutor(priorityUrls);
        this.clients = new Map(
            priorityUrls.map((p) => [p.url, new WrappedSuiClient(p.url, network)]),
        );
        this.timeoutMs = Number(process.env.RPC_REQUEST_TIMEOUT_MS) || 7000;
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
                logger.warn("RPC call failed", { url, error: formatError(error) });
                return {
                    status: "retry-same",
                    error: error instanceof Error ? error : new Error(String(error)),
                };
            }
        });
    }

    private isValidGetObjectResponse(suiObjectResponse: SuiObjectResponse): boolean {
        const data = suiObjectResponse.data;
        const error = suiObjectResponse.error;
        if (data) {
            return true;
        }
        if (error) {
            logger.warn("Failed to get object", { error: JSON.stringify(error) });
            return true;
        }
        return false;
    }

    private isValidMultiGetObjectResponse(suiObjectResponseArray: SuiObjectResponse[]): boolean {
        return suiObjectResponseArray.every((suiObjectResponse) => {
            return this.isValidGetObjectResponse(suiObjectResponse);
        });
    }

    public async getObject(input: GetObjectParams): Promise<SuiObjectResponse> {
        logger.info("RPCSelector: getObject", { input });
        const suiObjectResponse = await this.invokeWithFailover((client) =>
            client.getObject(input),
        );
        if (this.isValidGetObjectResponse(suiObjectResponse)) {
            return suiObjectResponse;
        }
        throw new Error("Invalid response from getObject.");
    }

    public async multiGetObjects(input: MultiGetObjectsParams): Promise<SuiObjectResponse[]> {
        logger.info("RPCSelector: multiGetObjects", { input });
        const suiObjectResponseArray = await this.invokeWithFailover((client) =>
            client.multiGetObjects(input),
        );
        if (this.isValidMultiGetObjectResponse(suiObjectResponseArray)) {
            return suiObjectResponseArray;
        }
        throw new Error("Invalid response from multiGetObjects.");
    }

    public async getDynamicFieldObject(
        input: GetDynamicFieldObjectParams,
    ): Promise<SuiObjectResponse> {
        logger.info("RPCSelector: getDynamicFieldObject", { input });
        const suiObjectResponse = await this.invokeWithFailover((client) =>
            client.getDynamicFieldObject(input),
        );
        if (this.isValidGetObjectResponse(suiObjectResponse)) {
            return suiObjectResponse;
        }
        throw new Error("Invalid response from getDynamicFieldObject.");
    }

    public async getNameRecord(name: string): Promise<NameRecord | null> {
        logger.info("RPCSelector: getNameRecord", { name });
        return this.invokeWithFailover((client) => client.getNameRecord(name));
    }
}
