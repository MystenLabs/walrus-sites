// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import {
    type WalrusSitesCompatibleClient,
    type CreateSiteOptions,
    type CreateAndAddResourceOptions,
} from '@types'
import { MissingRequiredWalrusClient, NotImplemented } from '@errors'
import * as site from 'contracts/sites/walrus_site/site'
import * as metadata from 'contracts/sites/walrus_site/metadata'
import { Transaction } from '@mysten/sui/transactions'
import { WalrusFile } from '@mysten/walrus'
import { Ed25519Keypair } from '@mysten/sui/keypairs/ed25519'

/**
 * Factory for extending a Sui client with Walrus Sites functionality.
 * Used along with `SuiClient.$extend(walrusSites())`.
 *
 * @returns An extension descriptor with a `name` and a `register` function
 * that produces a `WalrusSitesClient` when given a `WalrusSitesCompatibleClient`.
 */
export function walrusSites() {
    return {
        name: 'walrus_sites',
        register: (extendedSuiClient: WalrusSitesCompatibleClient) => {
            if (!extendedSuiClient.walrus) {
                throw MissingRequiredWalrusClient
            }
            return new WalrusSitesClient(extendedSuiClient)
        },
    }
}

/**
 * The WalrusSitesClient. Use this to interact with the Walrus Sites smart contract.
 */
export class WalrusSitesClient {
    #extendedSuiClient: WalrusSitesCompatibleClient
    constructor(extendedSuiClient: WalrusSitesCompatibleClient) {
        this.#extendedSuiClient = extendedSuiClient
    }

    // Top level methods.
    // WARNING: When using the walrus SDK without an upload relay, it is important to understand that reading and
    // writing walrus blobs requires a lot of requests (~2200 to write a blob, ~335 to read a blob).
    public async publish(
        args: {
            files: { path: string; contents: Uint8Array }[]
            siteOptions: CreateSiteOptions
        },
        keypair: Ed25519Keypair
    ) {
        // TODO(alex): Maybe we should add an upper limit to avoid moveCall PTB overflow.
        // Currently, the provided files will all be written into a single quilt.
        // Future versions of the Walrus SDK may optimize how files are stored to be more efficient
        // by splitting files into multiple quilts.
        const walrusFiles = args.files.map((file) =>
            WalrusFile.from({
                contents: file.contents,
                identifier: file.path,
            })
        )
        const storeOnWalrusResult = await this.#extendedSuiClient.walrus.writeFiles({
            files: walrusFiles,
            epochs: 3,
            deletable: true,
            signer: keypair,
        })
        console.log(storeOnWalrusResult)
        // Steps:
        // 0. publish files to Walrus as quilts pseudocode: files == [LocalResources {buffer: Bytes, metadata...})].
        // 1. create site
        // 2. attach routes
        // 3. create_resource
        // throw new NotImplemented()
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
    // The upload relay will reduce the number of requests needed to write a blob, but reads through
    // the walrus SDK will still require a lot of requests.
    public view = {
        sitemap: () => {
            throw new NotImplemented()
        },
    }

    // PTB construction.
    public tx = {
        /**
         * Generates a Transaction that creates a site and sends it to an address.
         * @param transaction Optional existing Transaction instance to add commands to. If not provided, a new Transaction will be created.
         * @param args Arguments for site creation, including the site name, recipient address, and optional metadata.
         * @returns The Transaction containing all commands necessary to create and transfer the site object.
         */
        createSite: (transaction = new Transaction(), args: CreateSiteOptions) => {
            const metadataObj = metadata.newMetadata({
                arguments: {
                    link: args.siteMetadata?.link ?? null,
                    imageUrl: args.siteMetadata?.image_url ?? null,
                    description: args.siteMetadata?.description ?? null,
                    projectUrl: args.siteMetadata?.project_url ?? null,
                    creator: args.siteMetadata?.creator ?? null,
                },
            })
            const site_object = site.newSite({
                arguments: [transaction.pure.string(args.siteName), metadataObj],
            })
            const res = transaction.add(site_object)
            transaction.transferObjects([res], args.owner)
            return transaction
        },
        /**
         * Adds commands to create a site resource (and optional headers) to a
         * `Transaction` (or a new one if none is provided).
         * @param transaction Existing `Transaction` or a new one is created.
         * @param args Options for the resource: range, resource fields, and headers.
         * @returns The `Transaction` with all resource-related commands added.
         */
        createAndAddResource: (
            transaction = new Transaction(),
            args: CreateAndAddResourceOptions
        ) => {
            const range = this.call.newRange(args.newRangeOptions)
            const resource = this.call.newResource({
                arguments: {
                    ...args.newResourceArguments,
                    range,
                },
            })
            transaction.add(resource)
            for (const [key, value] of Object.entries(args.resourceHeaders ?? {})) {
                const header = this.call.addHeader({
                    arguments: {
                        resource,
                        name: key,
                        value,
                    },
                })
                transaction.add(header)
            }
            transaction.add(
                this.call.addResource({ arguments: { ...args.addResourceArguments, resource } })
            )
            return transaction
        },
        removeResource: () => {
            throw new NotImplemented()
        },
        createRoutes: () => {
            throw new NotImplemented()
        },
        removeRoutes: () => {
            throw new NotImplemented()
        },
        destroySite: () => {
            throw new NotImplemented()
        },
    }

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
        },
    }
}
