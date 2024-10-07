// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

export const NETWORK = "testnet"
export const AGGREGATOR = "https://aggregator-devnet.walrus.space:443"
export const SITE_PACKAGE = "0xe15cd956d3f54ad0b6608b01b96e9999d66552dfd025e698ac16cd0df1787a25"
export const MAX_REDIRECT_DEPTH = 3
export const SITE_NAMES: { [key: string]: string } = {
    // Any hardcoded (non suins) name -> object_id mappings go here
    // e.g.,
    // landing: "0x1234..."
};
// The default portal to redirect to if the browser does not support service workers.
export const FALLBACK_PORTAL = "blob.store"
// The string representing the ResourcePath struct in the walrus_site package.
export const RESOURCE_PATH_MOVE_TYPE = SITE_PACKAGE + "::site::ResourcePath";
