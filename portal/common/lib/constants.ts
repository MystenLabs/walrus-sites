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

const LANDING_PAGE_OID = '0x2d9414edc309535bfd4cd7e80ccbc09fee18bf86b449a185b81e914096059a67';
const FLATLAND_OID = '0xf60797491f9303de69856b7d2fc1109daf63450ec8cd7fb49f1bd4a0e7d26ae6';
const FLATLANDER_OID = '0xd2de62949d832aea46b0eac830d9837885d419ba5b2baa7f2b95d10059573ddf';
export const SITES_USED_FOR_BENCHING = [
    [LANDING_PAGE_OID, "landing page"],
    [FLATLAND_OID, "flatland"],
    [FLATLANDER_OID, "flatlander"]
]
