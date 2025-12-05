// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { type WalrusSitesCompatibleClient } from "@types";
import { MissingRequiredWalrusClient } from "@errors";


function walrusSites() {
	return {
		name: 'walrus_sites',
		register: (extendedSuiClient: WalrusSitesCompatibleClient) => {
		    if (!extendedSuiClient.walrus) {
				throw MissingRequiredWalrusClient
			}
			return new WalrusSitesClient(extendedSuiClient);
		}
	}
}

export class WalrusSitesClient {
    #extendedSuiClient: WalrusSitesCompatibleClient;
    constructor(extendedSuiClient: WalrusSitesCompatibleClient) {
        this.#extendedSuiClient = extendedSuiClient
    }

}
