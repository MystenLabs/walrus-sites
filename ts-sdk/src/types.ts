// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0
import { type ClientWithExtensions, type Experimental_CoreClient } from "@mysten/sui/experimental";
import { WalrusClient } from "@mysten/walrus";
import * as siteContract from "contracts/sites/walrus_site/site";
import * as metadataContract from "contracts/sites/walrus_site/metadata";
import * as eventsContract from "contracts/sites/walrus_site/events";

export type WalrusSitesCompatibleClient = ClientWithExtensions<{
	core: Experimental_CoreClient;
	walrus: WalrusClient;
}>;

// Site module types
export type Site = (typeof siteContract.Site)['$inferType'];
export type Range = (typeof siteContract.Range)['$inferType'];
export type Resource = (typeof siteContract.Resource)['$inferType'];
export type ResourcePath = (typeof siteContract.ResourcePath)['$inferType'];
export type Routes = (typeof siteContract.Routes)['$inferType'];

// Metadata module types
export type Metadata = (typeof metadataContract.Metadata)['$inferType'];

// Events module types
export type SiteCreatedEvent = (typeof eventsContract.SiteCreatedEvent)['$inferType'];
export type SiteBurnedEvent = (typeof eventsContract.SiteBurnedEvent)['$inferType'];

// Walrus Sites Client types
export type CreateSiteOptions = {siteName: string, sendSiteToAddress: string, siteMetadata?: Metadata}
