// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import {
    type WalrusSitesCompatibleClient,
    type CreateSiteOptions,
    type CreateAndAddResourceOptions,
    type File,
    type QuiltPatch,
} from '@types'
import { MissingRequiredWalrusClientError, NotImplementedError } from '@errors'
import * as siteModule from 'contracts/sites/walrus_site/site'
import * as metadataModule from 'contracts/sites/walrus_site/metadata'
import { Transaction } from '@mysten/sui/transactions'
import { WalrusFile } from '@mysten/walrus'
import { Ed25519Keypair } from '@mysten/sui/keypairs/ed25519'
import { sha256, toQuiltPatchIdHex, QUILT_PATCH_ID_INTERNAL_HEADER } from '@utils'

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
                throw MissingRequiredWalrusClientError
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

    // WARNING: When using the walrus SDK without an upload relay, it is important to understand that reading and
    // writing walrus blobs requires a lot of requests (~2200 to write a blob, ~335 to read a blob).
    public async publish(
        args: {
            files: File[]
            siteOptions: CreateSiteOptions
        },
        epochs: number,
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
        const blobs = await this.#extendedSuiClient.walrus.writeFiles({
            files: walrusFiles,
            epochs,
            deletable: true,
            signer: keypair,
        })
        if (blobs.length != args.files.length) {
            throw new Error() // TODO Add custom error
        }
        const transaction = new Transaction()
        const metadataObj = this.call.newMetadata({
            arguments: {
                link: args.siteOptions.siteMetadata?.link ?? null,
                imageUrl: args.siteOptions.siteMetadata?.image_url ?? null,
                description: args.siteOptions.siteMetadata?.description ?? null,
                projectUrl: args.siteOptions.siteMetadata?.project_url ?? null,
                creator: args.siteOptions.siteMetadata?.creator ?? null,
            },
        })

        // TODO: If we can get the site object from inside a transaction,
        // we can call this.tx.createSite instead of this.call.newSite.
        const site = this.call.newSite({
            arguments: {
                name: args.siteOptions.siteName,
                metadata: metadataObj,
            },
        })
        // TODO: do we want to send it to the sender's address?
        transaction.transferObjects([site], keypair.toSuiAddress())

        const zipped = args.files.map((file, i): [File, QuiltPatch] => {
            const blob = blobs[i]
            if (file && blob) {
                return [file, blob]
            }
            throw new Error() // TODO Add custom error
        })
        for (const [file, blob] of zipped) {
            const blobHash = await sha256(Buffer.from(file.contents))
            // TODO: Find a clean way to construct null ranges.
            const range = this.call.newRange({ arguments: { rangeStart: 0, rangeEnd: 1 } })
            this.tx.createAndAddResource(transaction, {
                site,
                newResourceArguments: {
                    path: file!.path,
                    blobId: Number(blob.blobId),
                    blobHash: 123, // TODO: Buffer.from(blobHash).readBigInt64BE(),
                    range,
                },
                newRangeOptions: {
                    // This newRangeOptions overwrites the range above.
                    arguments: {
                        rangeStart: 0, // TODO
                        rangeEnd: 0, // TODO
                    },
                },
                resourceHeaders: file.headers?.set(
                    QUILT_PATCH_ID_INTERNAL_HEADER,
                    toQuiltPatchIdHex(blob.id)
                ),
            })
        }
        const bytes = await transaction.build()
        const signature = await transaction.sign({ signer: keypair })
        return await this.#extendedSuiClient.core.executeTransaction({
            transaction: bytes,
            signatures: [signature.bytes],
        })
    }

    public update() {
        throw new NotImplementedError()
    }

    public destroy() {
        throw new NotImplementedError()
    }

    public updateResources() {
        throw new NotImplementedError()
    }

    // Data fetching functions.
    // The upload relay will reduce the number of requests needed to write a blob, but reads through
    // the walrus SDK will still require a lot of requests.
    public view = {
        sitemap: () => {
            throw new NotImplementedError()
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
            const metadataObj = this.call.newMetadata({
                arguments: {
                    link: args.siteMetadata?.link ?? null,
                    imageUrl: args.siteMetadata?.image_url ?? null,
                    description: args.siteMetadata?.description ?? null,
                    projectUrl: args.siteMetadata?.project_url ?? null,
                    creator: args.siteMetadata?.creator ?? null,
                },
            })
            const site_object = this.call.newSite({
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
            for (const [key, value] of Object.entries(args.resourceHeaders ?? {})) {
                this.call.addHeader({
                    arguments: {
                        resource,
                        name: key,
                        value,
                    },
                })
            }
            transaction.add(this.call.addResource({ arguments: { site: args.site, resource } }))
            return transaction
        },
        removeResource: () => {
            throw new NotImplementedError()
        },
        createRoutes: () => {
            throw new NotImplementedError()
        },
        removeRoutes: () => {
            throw new NotImplementedError()
        },
        destroySite: () => {
            throw new NotImplementedError()
        },
    }

    // Direct move calls to the contract.
    public call = {
        newSite: (args: siteModule.NewSiteOptions) => {
            return siteModule.newSite(args)
        },
        newRangeOption: (args: siteModule.NewRangeOptionOptions) => {
            return siteModule.newRangeOption(args)
        },
        newRange: (args: siteModule.NewRangeOptions) => {
            return siteModule.newRange(args)
        },
        newResource: (args: siteModule.NewResourceOptions) => {
            return siteModule.newResource(args)
        },
        addHeader: (args: siteModule.AddHeaderOptions) => {
            return siteModule.addHeader(args)
        },
        updateName: (args: siteModule.UpdateNameOptions) => {
            return siteModule.updateName(args)
        },
        updateMetadata: (args: siteModule.UpdateMetadataOptions) => {
            return siteModule.updateMetadata(args)
        },
        addResource: (args: siteModule.AddResourceOptions) => {
            return siteModule.addResource(args)
        },
        removeResource: (args: siteModule.RemoveResourceOptions) => {
            return siteModule.removeResource(args)
        },
        removeResourceIfExists: (args: siteModule.RemoveResourceIfExistsOptions) => {
            return siteModule.removeResourceIfExists(args)
        },
        moveResource: (args: siteModule.MoveResourceOptions) => {
            return siteModule.moveResource(args)
        },
        createRoutes: (args: siteModule.CreateRoutesOptions) => {
            return siteModule.createRoutes(args)
        },
        removeAllRoutesIfExist: (args: siteModule.RemoveAllRoutesIfExistOptions) => {
            return siteModule.removeAllRoutesIfExist(args)
        },
        insertRoute: (args: siteModule.InsertRouteOptions) => {
            return siteModule.insertRoute(args)
        },
        removeRoute: (args: siteModule.RemoveRouteOptions) => {
            return siteModule.removeRoute(args)
        },
        burn: (args: siteModule.BurnOptions) => {
            return siteModule.burn(args)
        },
        newMetadata: (args: metadataModule.NewMetadataOptions) => {
            return metadataModule.newMetadata(args)
        },
    }
}
