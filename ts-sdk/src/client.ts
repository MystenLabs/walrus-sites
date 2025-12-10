// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { type WalrusSitesCompatibleClient, type Metadata } from "@types";
import { MissingRequiredWalrusClient, NotImplemented } from "@errors";
import * as site from "contracts/sites/walrus_site/site";
import * as metadata from "contracts/sites/walrus_site/metadata";
import { Transaction } from "@mysten/sui/transactions";

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

    // Top level methods.
    public publish() {

        throw new NotImplemented()
    }

    public update() {
        throw new NotImplemented()
    }

    public destroy() {
        throw new NotImplemented()
    }

    public updateResources() {
        throw new NotImplemented()
    }

    // Data fetching functions.
    public view = {
        sitemap: () => { throw new NotImplemented() }
    }

    // PTB construction.
    public tx = {
        createSite: (transaction = new Transaction(), args: {siteName: string, sendSiteToAddress: string, siteMetadata?: Metadata}) => {
            const metadataObj = metadata.newMetadata({
                arguments: {
                    link: args.siteMetadata?.link ?? null,
                    imageUrl: args.siteMetadata?.link ?? null,
                    description: args.siteMetadata?.link ?? null,
                    projectUrl: args.siteMetadata?.link ?? null,
                    creator: args.siteMetadata?.link ?? null,
                },
            })
            const site_object = site.newSite({arguments: [transaction.pure.string(args.siteName), metadataObj]})
            const res = transaction.add(site_object)
            transaction.transferObjects([res], args.sendSiteToAddress)
            return transaction
        },
        // Create a resource with ranges, create headers and attach to resource, add_resource to site.
        createResource: () => { throw new NotImplemented() },
        createRoutes: () => { throw new NotImplemented() },
        removeResource: () => { throw new NotImplemented() },
        removeRoutes: () => { throw new NotImplemented() },
        destroySite: () => { throw new NotImplemented() }
    };

    // Direct move calls to the contract.
    public call = {
        newSite: (args: site.NewSiteOptions) => {
            return site.newSite(args)
        },
        newRangeOption: (args: site.NewRangeOptionOptions) => {
            return site.newRangeOption(args)
        },
        newRange: (args: site.NewRangeOptions) => {
            return site.newRange(args)
        },
        newResource: (args: site.NewResourceOptions) => {
            return site.newResource(args)
        },
        addHeader: (args: site.AddHeaderOptions) => {
            return site.addHeader(args)
        },
        updateName: (args: site.UpdateNameOptions) => {
            return site.updateName(args)
        },
        updateMetadata: (args: site.UpdateMetadataOptions) => {
            return site.updateMetadata(args)
        },
        addResource: (args: site.AddResourceOptions) => {
            return site.addResource(args)
        },
        removeResource: (args: site.RemoveResourceOptions) => {
            return site.removeResource(args)
        },
        removeResourceIfExists: (args: site.RemoveResourceIfExistsOptions) => {
            return site.removeResourceIfExists(args)
        },
        moveResource: (args: site.MoveResourceOptions) => {
            return site.moveResource(args)
        },
        createRoutes: (args: site.CreateRoutesOptions) => {
            return site.createRoutes(args)
        },
        removeAllRoutesIfExist: (args: site.RemoveAllRoutesIfExistOptions) => {
            return site.removeAllRoutesIfExist(args)
        },
        insertRoute: (args: site.InsertRouteOptions) => {
            return site.insertRoute(args)
        },
        removeRoute: (args: site.RemoveRouteOptions) => {
            return site.removeRoute(args)
        },
        burn: (args: site.BurnOptions) => {
            return site.burn(args)
        },
        newMetadata: (args: metadata.NewMetadataOptions) => {
            return metadata.newMetadata(args)
        }
    };
}
