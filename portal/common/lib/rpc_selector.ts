// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import {
    GetDynamicFieldObjectParams,
    GetObjectParams,
    MultiGetObjectsParams,
    SuiClient,
    SuiObjectResponse,
} from "@mysten/sui/client";
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

export class RPCSelector implements RPCSelectorInterface {
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
            setTimeout(() => reject(
                new Error("Request timed out")),
                Number(process.env.RPC_REQUEST_TIMEOUT_MS) ?? 7000
            ),
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

        const isValid = methodName === "call" && args[0] === "suix_resolveNameServiceAddress" && this.isValidSuiNSResponse(result)
        const isValidGetObject = methodName === "getObject" && this.isValidGetObjectResponse(result);
        const isValidMultiGetObject = methodName === "multiGetObjects" && this.isValidMultiGetObjectResponse(result);
        const isValidDynamicFieldObject = methodName === "getDynamicFieldObject" && this.isValidGetObjectResponse(result);
        if (isValid || isValidGetObject || isValidMultiGetObject || isValidDynamicFieldObject) {
            return result
        }
        throw new Error("Invalid response from selected client");
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

                    // TODO: Refactor until L:120
                    const isValid = methodName === "call" && args[0] === "suix_resolveNameServiceAddress" && this.isValidSuiNSResponse(result)
                    const isValidGetObject = methodName === "getObject" && this.isValidGetObjectResponse(result);
                    const isValidMultiGetObject = methodName === "multiGetObjects" && this.isValidMultiGetObjectResponse(result);
                    const isValidDynamicFieldObject = methodName === "getDynamicFieldObject" && this.isValidGetObjectResponse(result);
                    if (isValid || isValidGetObject || isValidMultiGetObject || isValidDynamicFieldObject) {
                        resolve({ result, client });
                    }
                    reject(new Error(`Invalid response for methodName: ${methodName} and args: ${args}`));
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
        } catch (error) {
            throw new Error(`Failed to contact fallback RPC clients.`);
        }
    }

    private isValidGetObjectResponse(suiObjectResponse: SuiObjectResponse): boolean {
        const data = suiObjectResponse.data;
        const error = suiObjectResponse.error;
        if (data) {
            return true;
        }
        if (error) {
            logger.warn({message: 'Failed to get object', error: error})
            return true
        }
        return false
    }

    private isValidMultiGetObjectResponse(suiObjectResponseArray: SuiObjectResponse[]): boolean {
       return suiObjectResponseArray.every((suiObjectResponse) => {
           return this.isValidGetObjectResponse(suiObjectResponse);
       });
    }

    private isValidSuiNSResponse(suinsResponse?: string): boolean {
        return suinsResponse ? true : false // FIXME: Implement this
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
