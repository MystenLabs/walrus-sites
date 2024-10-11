// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

export const NETWORK = "testnet";
export const AGGREGATOR = "https://aggregator-devnet.walrus.space:443";
export const SITE_PACKAGE = "0xbb9ab98d52664e4492db6eda5540db79e5778f2644eff11972aa80308265816b";
export const MAX_REDIRECT_DEPTH = 3;
export const SITE_NAMES: { [key: string]: string } = {
    // Any hardcoded (non suins) name -> object_id mappings go here
    // e.g.,
    // landing: "0x1234..."
};
// The default portal to redirect to if the browser does not support service workers.
export const FALLBACK_DEVNET_PORTAL = "blocksite.net";
// The string representing the ResourcePath struct in the walrus_site package.
export const RESOURCE_PATH_MOVE_TYPE = SITE_PACKAGE + "::site::ResourcePath";
