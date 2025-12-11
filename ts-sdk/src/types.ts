// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0
import { type ClientWithExtensions, type Experimental_CoreClient } from '@mysten/sui/experimental'
import { WalrusClient } from '@mysten/walrus'
import * as siteContract from 'contracts/sites/walrus_site/site'
import * as metadataContract from 'contracts/sites/walrus_site/metadata'
import * as eventsContract from 'contracts/sites/walrus_site/events'
import { type RawTransactionArgument } from 'contracts/sites/utils'

export type WalrusSitesCompatibleClient = ClientWithExtensions<{
    core: Experimental_CoreClient
    walrus: WalrusClient
}>

// Site module types
export type Site = (typeof siteContract.Site)['$inferType']
export type Range = (typeof siteContract.Range)['$inferType']
export type Resource = (typeof siteContract.Resource)['$inferType']
export type ResourcePath = (typeof siteContract.ResourcePath)['$inferType']
export type Routes = (typeof siteContract.Routes)['$inferType']

// Metadata module types
export type Metadata = (typeof metadataContract.Metadata)['$inferType']

// Events module types
export type SiteCreatedEvent = (typeof eventsContract.SiteCreatedEvent)['$inferType']
export type SiteBurnedEvent = (typeof eventsContract.SiteBurnedEvent)['$inferType']

// Walrus Sites Client types
export type CreateSiteOptions = {
    siteName: string
    owner: string
    siteMetadata?: Metadata
}
export type CreateAndAddResourceOptions = {
    newRangeOptions: siteContract.NewRangeOptions
    newResourceArguments: siteContract.NewResourceArguments
    site: RawTransactionArgument<string>
    resourceHeaders?: Map<string, string>
}
export type File = {
    path: string
    contents: Uint8Array
    headers?: Map<string, string>
    range?: Range
}
export type QuiltPatch = {
    id: string
    blobId: string
    blobObject: {
        id: {
            id: string
        }
        registered_epoch: number
        blob_id: string
        size: string
        encoding_type: number
        certified_epoch: number | null
        storage: {
            id: {
                id: string
            }
            start_epoch: number
            end_epoch: number
            storage_size: string
        }
        deletable: boolean
    }
}
