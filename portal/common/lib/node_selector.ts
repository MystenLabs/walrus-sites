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

// Interface defining the RPC selector methods.
interface RPCSelectorInterface {
    getObject(input: GetObjectParams): Promise<SuiObjectResponse>;
    multiGetObjects(input: MultiGetObjectsParams): Promise<SuiObjectResponse>;
    getDynamicFieldObject(
        input: GetDynamicFieldObjectParams,
    ): Promise<SuiObjectResponse>;
    call<T>(method: string, args: any[]): Promise<T>;
}

// Singleton RPC Selector class.
class RPCSelector implements RPCSelectorInterface {
    private static instance: RPCSelector;
    private clients: SuiClient[];
    private clientScores: Map<SuiClient, number>;

    private constructor(rpcURLs: string[]) {
        // Initialize clients immediately without blocking.
        this.clients = rpcURLs.map((rpcUrl) => new SuiClient({ url: rpcUrl }));
        // this.clientScores = new Map(this.clients.map((client) => [client, 0]));
    }

    // Get the singleton instance.
    public static getInstance(): RPCSelector {
        if (!RPCSelector.instance) {
        RPCSelector.instance = new RPCSelector(testnetRPCUrls);
        }
        return RPCSelector.instance;
    }

    // Update client's score based on success or failure.
    // private updateClientScore(client: SuiClient, success: boolean): void {
    //   const currentScore = this.clientScores.get(client) || 0;
    //   const newScore = success ? currentScore + 1 : currentScore - 1;
    //   this.clientScores.set(client, newScore);
    // }

    // Get clients sorted by their scores in descending order.
    // private getClientsByScore(): SuiClient[] {
    //   return [...this.clients]
    //     .filter((client) => this.clientHealth.get(client) !== false) // Exclude unhealthy clients
    //     .sort((a, b) => {
    //       const scoreA = this.clientScores.get(a) || 0;
    //       const scoreB = this.clientScores.get(b) || 0;
    //       return scoreB - scoreA;
    //     });
    // }

    // General method to call clients and return the first successful response.
    private async callClients<T>(methodName: string, args: any[]): Promise<T> {
        if (this.clients.length === 0) {
        throw new Error("No available clients to handle the request.");
        }

        const clientPromises = this.clients.map((client) =>
            new Promise<T>(async (resolve, reject) => {
            try {
                const method = (client as any)[methodName] as Function;
                if (!method) {
                reject(new Error(`Method ${methodName} not found on client`));
                return;
                }
                // TODO - verify that await is needed here
                // TODO => i.e. client.method(args)(?)
                const result = await method.apply(client, args);
                if (this.isValidResponse(result)) { // does this work?
                // this.updateClientScore(client, true);  - COMMENT a bit premature
                resolve(result);
                } else {
                // this.updateClientScore(client, false); - COMMENT a bit premature
                reject(new Error("Invalid response"));
                }
            } catch (error: any) {
                // Decrease score on any error, assuming node is down or unreachable.
                // this.updateClientScore(client, false); // COMMENT a bit premature
                reject(error);
            }
            }),
        );

        try {
        const result = await Promise.any(clientPromises);
        return result;
        } catch (errors) {
        throw new Error("All clients failed");
        }
    }

    // Check if the response is valid. -- FIXME
    private isValidResponse(result: any): boolean {
        console.log("res:", result);
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
        return this.callClients<SuiObjectResponse>("getDynamicFieldObject", [
        input,
        ]);
    }

    // Implementing generic call method.
    public async call<T>(method: string, args: any[]): Promise<T> {
        return this.callClients<T>(method, args);
    }
}

const rpcSelectorInstance = RPCSelector.getInstance();
export default rpcSelectorInstance;
