// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import {
    GetDynamicFieldObjectParams,
    GetObjectParams,
    MultiGetObjectsParams,
    SuiClient,
    SuiObjectResponse,
} from "@mysten/sui/client";
import { testnetRPCUrls } from "./constants";

interface RPCSelectorInterface {
    getObject(input: GetObjectParams): Promise<SuiObjectResponse>;
    multiGetObjects(input: MultiGetObjectsParams): Promise<SuiObjectResponse[]>;
    getDynamicFieldObject(
        input: GetDynamicFieldObjectParams,
    ): Promise<SuiObjectResponse>;
    call<T>(method: string, args: any[]): Promise<T>;
}

class RPCSelector implements RPCSelectorInterface {
    private static instance: RPCSelector;
    private clients: SuiClient[];
    private selectedClient: SuiClient | undefined;

    private constructor(rpcURLs: string[]) {
        // Initialize clients.
        this.clients = rpcURLs.map((rpcUrl) => new SuiClient({ url: rpcUrl }));
        this.selectedClient = undefined;
    }

    // Get the singleton instance.
    public static getInstance(): RPCSelector {
    if (!RPCSelector.instance) {
        RPCSelector.instance = new RPCSelector(testnetRPCUrls);
    }
    return RPCSelector.instance;
    }
    // General method to call clients and return the first successful response.
    private async callClients<T>(methodName: string, args: any[]): Promise<T> {
        if (this.clients.length === 0) {
            throw new Error("No available clients to handle the request.");
        }

        const isNoSelectedClient = !this.selectedClient;
        if (isNoSelectedClient) {
            return await this.callFallbackClients<T>(methodName, args);
        }

        try {
            return await this.callSelectedClient<T>(methodName, args);
        } catch (error) {
            console.error(`Selected client failed: ${error}`);
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
        const timeoutDuration = 5000;
        const timeoutPromise = new Promise<never>((_, reject) =>
            setTimeout(() => reject(new Error("Request timed out")), timeoutDuration),
        );

        const result = await Promise.race([
            method.apply(this.selectedClient, args),
            timeoutPromise,
        ]);

        if (this.isValidResponse(result)) {
            return result;
        } else {
            console.error("Invalid response from selected client");
            throw new Error("Invalid response from selected client");
        }
    }

    // Fallback to querying all clients using Promise.any.
    private async callFallbackClients<T>(methodName: string, args: any[]): Promise<T> {
        const clientPromises = this.clients.map((client) =>
            new Promise<{ result: T; client: SuiClient }>(async (resolve, reject) => {
                try {
                    const method = (client as any)[methodName] as Function;
                    if (!method) {
                        reject(new Error(`Method ${methodName} not found on client`));
                        return;
                    }
                    const result = await method.apply(client, args);
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
            return result;
        } catch {
            throw new Error("All clients failed");
        }
    }

    private isString(result: any): result is string {
        return typeof result === 'string';
    }

    private isValidResponse(result: SuiObjectResponse | SuiObjectResponse[]): boolean {
        if (!result) {
            return false;
        }
        if (this.isString(result)) {
            return result.trim().length > 0;
        } else if (Array.isArray(result)) {
            return result.some((item) => this.validateSuiObjectResponse(item));
        } else {
            return this.validateSuiObjectResponse(result);
        }
    }

    private validateSuiObjectResponse(response: SuiObjectResponse): boolean {
        if (response.error) {
            return false;
        }
        if (response.data) {
            return true;
        }
        return false;
    }

    public async getObject(input: GetObjectParams): Promise<SuiObjectResponse> {
        return this.callClients<SuiObjectResponse>("getObject", [input]);
    }

    public async multiGetObjects(
        input: MultiGetObjectsParams,
    ): Promise<SuiObjectResponse[]> {
        return this.callClients<SuiObjectResponse[]>("multiGetObjects", [input]);
    }

    public async getDynamicFieldObject(
        input: GetDynamicFieldObjectParams,
    ): Promise<SuiObjectResponse> {
        return this.callClients<SuiObjectResponse>("getDynamicFieldObject", [
        input,
        ]);
    }

    public async call<T>(method: string, args: any[]): Promise<T> {
        return this.callClients<T>(method, args);
    }
}

const rpcSelectorInstance = RPCSelector.getInstance();
export default rpcSelectorInstance;
