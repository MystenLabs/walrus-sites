// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

export const NETWORK = "testnet"
export const AGGREGATOR = "https://aggregator-devnet.walrus.space:443"
export const SITE_PACKAGE = "0x1ba588fd10c79e11a032e0947f454ced0a52f1a83c7fc4b1006bff548845e6c1"
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

const LANDING_PAGE_OID = '0x77fce72aa13df139ebdd605c56c1196ad5e62c7cb8236a8c6c1cbfc3be5c7de9';
const FLATLAND_OID = '0x72a45ae56b8e11c8f1cf8c83795af880cf76bc4a2f8fb165f41131018459849f';
const FLATLANDER_OID = '0x3dcce3b879a015f3e9ab8d48af76df333b4b64db59ac5a3bb8c19ff81c0ad586';
export const SITES_USED_FOR_BENCHING = [
    [LANDING_PAGE_OID, "landing page"],
    [FLATLAND_OID, "flatland"],
    // [FLATLANDER_OID, "flatlander"]
]
