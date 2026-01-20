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
import logger from "@lib/logger";
import { NameRecord, Network } from "@lib/types";

interface RPCSelectorInterface {
    getObject(input: GetObjectParams): Promise<SuiObjectResponse>;
    multiGetObjects(input: MultiGetObjectsParams): Promise<SuiObjectResponse[]>;
    getDynamicFieldObject(
        input: GetDynamicFieldObjectParams,
    ): Promise<SuiObjectResponse>;
}

class WrappedSuiClient extends SuiClient {
    private url: string;
    private suinsClient: SuinsClient;

    constructor(url: string, network: Network) {
        super({ url });
        this.url = url;
        this.suinsClient = new SuinsClient({
            client: this as SuiClient,
            network
        });
    }

    public getURL(): string {
        return this.url;
    }

    // Extends the SuiClient class to add a method to get a SuiNS record.
    // Useful for treating the SuiClient as a SuiNS client during the invokeWithFailover method.
    public async getNameRecord(name: string): Promise<NameRecord | null> {
        return await this.suinsClient.getNameRecord(name)
    }
}


export class RPCSelector implements RPCSelectorInterface {
    private clients: WrappedSuiClient[];
    private selectedClient: WrappedSuiClient | undefined;

    constructor(rpcURLs: string[], network: Network) {
        // Initialize clients.
        this.clients = rpcURLs.map((rpcUrl) => new WrappedSuiClient(rpcUrl, network));
        this.selectedClient = undefined;
    }

    // General method to call clients and return the first successful response.
    private async invokeWithFailover<T>(methodName: string, args: any[]): Promise<T> {
        if (this.clients.length === 0) {
            throw new Error("No available clients to handle the request.");
        }

        const isNoSelectedClient = !this.selectedClient;
        if (isNoSelectedClient) {
            logger.info("No selected RPC, looking for fallback...")
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
        if (!this.selectedClient) {
            throw new Error("Expected an RPC client to be selected");
        }
		logger.info("RPCSelector: Calling existing client", {
			clientName: this.selectedClient.getURL().toString(),
			methodName,
			arguments: args
		})
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
            logger.info(
                "Result null from current client",
                {nullCurrentRPCClientUrl: this.selectedClient.getURL().toString()})
        }

        return result
    }

    // Fallback to querying all clients using Promise.any.
	private async callFallbackClients<T>(methodName: string, args: any[]): Promise<T> {
		logger.info("RPCSelector: Calling fallback clients", { methodName, arguments: args })
        const clientPromises = this.clients.map((client) =>
            new Promise<{ result: T; client: WrappedSuiClient }>(async (resolve, reject) => {
                try {
                    const method = (client as any)[methodName] as Function;
                    if (!method) {
                        reject(new Error(`Method ${methodName} not found on client`));
                        return;
                    }
                    const result = await method.apply(client, args);
                    resolve({ result, client });
                } catch (error: any) {
                    reject(error);
                }
            }),
        );

        try {
            const { result, client } = await Promise.any(clientPromises);
            // Update the selected client for future calls.
            this.selectedClient = client;
            logger.info("RPCSelector: Client selected!", {
            	clientSelected: this.selectedClient.getURL().toString(),
             	methodName,
              	arguments: args
            })

            return result;
        } catch (error) {
            const message = `Failed to contact fallback RPC clients.`
            logger.error( message, { error: JSON.stringify(error) });
            throw new Error(message);
        }
    }

    private isValidGetObjectResponse(suiObjectResponse: SuiObjectResponse): boolean {
        const data = suiObjectResponse.data;
        const error = suiObjectResponse.error;
        if (data) {
            return true;
        }
        if (error) {
            logger.warn('Failed to get object', {error: JSON.stringify(error)})
            return true
        }
        return false
    }

    private isValidMultiGetObjectResponse(suiObjectResponseArray: SuiObjectResponse[]): boolean {
       return suiObjectResponseArray.every((suiObjectResponse) => {
           return this.isValidGetObjectResponse(suiObjectResponse);
       });
    }

    public async getObject(input: GetObjectParams): Promise<SuiObjectResponse> {
        const suiObjectResponse = await this.invokeWithFailover<SuiObjectResponse>("getObject", [input]);
        if (this.isValidGetObjectResponse(suiObjectResponse)) {
            return suiObjectResponse
        }
        throw new Error("Invalid response from getObject.");
    }

    public async multiGetObjects(
        input: MultiGetObjectsParams,
    ): Promise<SuiObjectResponse[]> {
        const suiObjectResponseArray = await this.invokeWithFailover<SuiObjectResponse[]>("multiGetObjects", [input]);
        if (this.isValidMultiGetObjectResponse(suiObjectResponseArray)) {
            return suiObjectResponseArray
        }
        throw new Error("Invalid response from multiGetObjects.");
    }

    public async getDynamicFieldObject(
        input: GetDynamicFieldObjectParams,
    ): Promise<SuiObjectResponse> {
        const suiObjectResponse = await this.invokeWithFailover<SuiObjectResponse>("getDynamicFieldObject", [
            input,
        ]);
        if (this.isValidGetObjectResponse(suiObjectResponse)) {
            return suiObjectResponse
        }
        throw new Error("Invalid response from getDynamicFieldObject.");
    }

    public async getNameRecord(name: string): Promise<NameRecord | null> {
        const nameRecord = await this.invokeWithFailover<NameRecord>('getNameRecord', [name])
        return nameRecord
    }
}
