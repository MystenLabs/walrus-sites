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
    multiGetObjects(input: MultiGetObjectsParams): Promise<SuiObjectResponse>;
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

        // Check if a selected client exists
        if (this.selectedClient) {
            try {
                const method = (this.selectedClient as any)[methodName] as Function;
                if (!method) {
                    throw new Error(`Method ${methodName} not found on selected client`);
                }
                const result = await method.apply(this.selectedClient, args);
                if (this.isValidResponse(result)) {
                    return result;
                } else {
                    console.error("Invalid response from selected client");
                    // Unset the selected client to trigger fallback
                    this.selectedClient = undefined;
                }
            } catch (error) {
                console.error(`Error with selected client: ${error}`);
                // Unset the selected client to trigger fallback
                this.selectedClient = undefined;
            }
        }

        // Fallback to querying all clients using Promise.any
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
            // Update the selected client
            this.selectedClient = client;
            return result;
        } catch (errors) {
            throw new Error("All clients failed");
        }
    }

    // Check if the response is valid. -- FIXME
    private isValidResponse(result: any): boolean {
        return result !== null && result !== undefined;
    }

    // Implementing getObject method.
    public async getObject(input: GetObjectParams): Promise<SuiObjectResponse> {
        return this.callClients<SuiObjectResponse>("getObject", [input]);
    }

    // Implementing multiGetObjects method.
    public async multiGetObjects(
        input: MultiGetObjectsParams,
    ): Promise<SuiObjectResponse> {
        return this.callClients<SuiObjectResponse>("multiGetObjects", [input]);
    }

    // Implementing getDynamicFieldObject method.
    public async getDynamicFieldObject(
        input: GetDynamicFieldObjectParams,
    ): Promise<SuiObjectResponse> {
        return this.callClients<SuiObjectResponse>("getDynamicFieldObject", [input]);
    }

    // Implementing generic call method.
    public async call<T>(method: string, args: any[]): Promise<T> {
        return this.callClients<T>(method, args);
    }
}

const rpcSelectorInstance = RPCSelector.getInstance();
export default rpcSelectorInstance;
