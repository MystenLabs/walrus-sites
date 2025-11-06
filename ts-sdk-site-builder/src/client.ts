// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { SuiClient } from "@mysten/sui/client";

export interface ClientConfig {
    suiClient: SuiClient;
}

export class Client {
    readonly suiClient: SuiClient;

    constructor(config: ClientConfig) {
        this.suiClient = config.suiClient;
    }
}
