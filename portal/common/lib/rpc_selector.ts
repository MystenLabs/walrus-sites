// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import {
    GetDynamicFieldObjectParams,
    GetObjectParams,
    MultiGetObjectsParams,
    SuiClient,
    SuiObjectResponse,
} from "@mysten/sui/client";
import { RPC_REQUEST_TIMEOUT_MS } from "./constants";
import logger from "./logger";

interface RPCSelectorInterface {
    getObject(input: GetObjectParams): Promise<SuiObjectResponse>;
    multiGetObjects(input: MultiGetObjectsParams): Promise<SuiObjectResponse[]>;
    getDynamicFieldObject(
        input: GetDynamicFieldObjectParams,
    ): Promise<SuiObjectResponse>;
    call<T>(method: string, args: any[]): Promise<T>;
}

class WrappedSuiClient extends SuiClient {
    private url: string;

    constructor(url: string) {
        super({ url });
        this.url = url;
    }

    public getURL(): string {
        return this.url;
    }
}

class RPCSelector implements RPCSelectorInterface {
    private static instance: RPCSelector;
    private clients: WrappedSuiClient[];
    private selectedClient: WrappedSuiClient | undefined;

    constructor(rpcURLs: string[]) {
        // Initialize clients.
        this.clients = rpcURLs.map((rpcUrl) => new WrappedSuiClient(rpcUrl));
        this.selectedClient = undefined;
    }

    // General method to call clients and return the first successful response.
    private async invokeWithFailover<T>(methodName: string, args: any[]): Promise<T> {
        if (this.clients.length === 0) {
            throw new Error("No available clients to handle the request.");
        }

        const isNoSelectedClient = !this.selectedClient;
        if (isNoSelectedClient) {
            logger.info({message: "No selected RPC, looking for fallback..."})
            return await this.callFallbackClients<T>(methodName, args);
        }

        try {
            return await this.callSelectedClient<T>(methodName, args);
        } catch (error) {
            this.selectedClient = undefined;
            return await this.callFallbackClients<T>(methodName, args);
        }
    }

    // Attempt to call the method on the selected client with a timeout.
    private async callSelectedClient<T>(methodName: string, args: any[]): Promise<T> {
        const method = (this.selectedClient as any)[methodName] as Function;
        if (!method) {
            throw new Error(`Method ${methodName} not found on selected client`);
        }

        const timeoutPromise = new Promise<never>((_, reject) =>
            setTimeout(() => reject(new Error("Request timed out")), RPC_REQUEST_TIMEOUT_MS),
        );

        const result = await Promise.race([
            method.apply(this.selectedClient, args),
            timeoutPromise,
        ]);

        if (result == null && this.selectedClient) {
            logger.info({
                message: "Result null from current client",
                nullCurrentRPCClientUrl: this.selectedClient.getURL().toString()})
        }

        if (this.isValidResponse(result)) {
            return result;
        } else {
            throw new Error("Invalid response from selected client");
        }
    }

    // Fallback to querying all clients using Promise.any.
    private async callFallbackClients<T>(methodName: string, args: any[]): Promise<T> {
        const clientPromises = this.clients.map((client) =>
            new Promise<{ result: T; client: WrappedSuiClient }>(async (resolve, reject) => {
                try {
                    const method = (client as any)[methodName] as Function;
                    if (!method) {
                        reject(new Error(`Method ${methodName} not found on client`));
                        return;
                    }
                    const result = await method.apply(client, args);
                    if (result == null) {
                        logger.info({
                            message: "Result null from fallback client:",
                            nullFallbackRPCClientUrl: client.getURL().toString()})
                    }
                    if (this.isValidResponse(result)) {
                        resolve({ result, client });
                    } else {
                        reject(new Error("Invalid response"));
                    }
                } catch (error: any) {
                    reject(error);
                }
            }),
        );

        try {
            const { result, client } = await Promise.any(clientPromises);
            // Update the selected client for future calls.
            this.selectedClient = client;
            logger.info({ message: "RPC selected", rpcClientSelected: this.selectedClient.getURL() })

            return result;
        } catch {
            throw new Error(`Failed to contact fallback RPC clients.`);
        }
    }

    private isValidResponse(result: SuiObjectResponse | SuiObjectResponse[] | string): boolean {
        // GetObject or getDynamicFieldObject
        if (result == null) {
            return false;
        }

        // SuiNS
        if (typeof result === 'string') {
            return result.trim().length > 0;
        }

        // MultiGetObject
        if (Array.isArray(result)) {
            return result.some((item) => item != null);
        }
        return true;
    }

    public async getObject(input: GetObjectParams): Promise<SuiObjectResponse> {
        return this.invokeWithFailover<SuiObjectResponse>("getObject", [input]);
    }

    public async multiGetObjects(
        input: MultiGetObjectsParams,
    ): Promise<SuiObjectResponse[]> {
        return this.invokeWithFailover<SuiObjectResponse[]>("multiGetObjects", [input]);
    }

    public async getDynamicFieldObject(
        input: GetDynamicFieldObjectParams,
    ): Promise<SuiObjectResponse> {
        return this.invokeWithFailover<SuiObjectResponse>("getDynamicFieldObject", [
            input,
        ]);
    }

    public async call<T>(method: string, args: any[]): Promise<T> {
        return this.invokeWithFailover<T>(method, args);
    }
}

if (!process.env.RPC_URL_LIST) {
    throw new Error("Missing TESTNET_RPC_LIST environment variable");
}
const rpcSelector = new RPCSelector(process.env.RPC_URL_LIST.split(','));
export default rpcSelector;
