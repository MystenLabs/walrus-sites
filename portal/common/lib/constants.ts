// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

export const NETWORK = "testnet"
export const AGGREGATOR = "https://aggregator-devnet.walrus.space:443"
export const SITE_PACKAGE = "0x514cf7ce2df33b9e2ca69e75bc9645ef38aca67b6f2852992a34e35e9f907f58"
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

const LANDING_PAGE_OID = '0x5fa99da7c4af9e2e2d0fb4503b058b9181693e463998c87c40be78fa2a1ca271';
const FLATLAND_OID = '0x049b6d3f34789904efcc20254400b7dca5548ee35cd7b5b145a211f85b2532fa';
export const SITES_USED_FOR_BENCHING = [
    [LANDING_PAGE_OID, "landing page"],
    [FLATLAND_OID, "flatland"]
]
