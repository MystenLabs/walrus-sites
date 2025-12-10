// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { type WalrusSitesCompatibleClient } from "@types";
import { MissingRequiredWalrusClient } from "@errors";
import * as contract from "contracts/sites/walrus_site/site";

/**
 * A function used to extend a Sui base client.
 * @returns An instance of the WalrusSitesClient
 */
export function walrusSites() {
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

/**
 * The WalrusSitesClient. Use this to interact with the Walrus Sites smart contract.
 */
export class WalrusSitesClient {
    #extendedSuiClient: WalrusSitesCompatibleClient;
    constructor(extendedSuiClient: WalrusSitesCompatibleClient) {
        this.#extendedSuiClient = extendedSuiClient
    }

    public call = {
        newSite: (args: contract.NewSiteOptions) => {
            return contract.newSite(args)
        },
        newRangeOption: (args: contract.NewRangeOptionOptions) => {
            return contract.newRangeOption(args)
        },
        newRange: (args: contract.NewRangeOptions) => {
            return contract.newRange(args)
        },
        newResource: (args: contract.NewResourceOptions) => {
            return contract.newResource(args)
        },
        addHeader: (args: contract.AddHeaderOptions) => {
            return contract.addHeader(args)
        },
        updateName: (args: contract.UpdateNameOptions) => {
            return contract.updateName(args)
        },
        updateMetadata: (args: contract.UpdateMetadataOptions) => {
            return contract.updateMetadata(args)
        },
        addResource: (args: contract.AddResourceOptions) => {
            return contract.addResource(args)
        },
        removeResource: (args: contract.RemoveResourceOptions) => {
            return contract.removeResource(args)
        },
        removeResourceIfExists: (args: contract.RemoveResourceIfExistsOptions) => {
            return contract.removeResourceIfExists(args)
        },
        moveResource: (args: contract.MoveResourceOptions) => {
            return contract.moveResource(args)
        },
        createRoutes: (args: contract.CreateRoutesOptions) => {
            return contract.createRoutes(args)
        },
        removeAllRoutesIfExist: (args: contract.RemoveAllRoutesIfExistOptions) => {
            return contract.removeAllRoutesIfExist(args)
        },
        insertRoute: (args: contract.InsertRouteOptions) => {
            return contract.insertRoute(args)
        },
        removeRoute: (args: contract.RemoveRouteOptions) => {
            return contract.removeRoute(args)
        },
        burn: (args: contract.BurnOptions) => {
            return contract.burn(args)
        },
        getSiteName: (args: contract.GetSiteNameOptions) => {
            return contract.getSiteName(args)
        },
        getSiteLink: (args: contract.GetSiteLinkOptions) => {
            return contract.getSiteLink(args)
        },
        getSiteImageUrl: (args: contract.GetSiteImageUrlOptions) => {
            return contract.getSiteImageUrl(args)
        },
        getSiteDescription: (args: contract.GetSiteDescriptionOptions) => {
            return contract.getSiteDescription(args)
        },
        getSiteProjectUrl: (args: contract.GetSiteProjectUrlOptions) => {
            return contract.getSiteProjectUrl(args)
        },
        getSiteCreator: (args: contract.GetSiteCreatorOptions) => {
            return contract.getSiteCreator(args)
        },
    };
}
