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
const FLATLAND_OID = '0xc62fae899d75705d88ef282678d17abc08a3363293def8841f0113aabd053fbb';
const FLATLANDER_OID = '0xabf413f36aa8ba984f81f4d3e334070b351c800dacb5ea5e02d49a7621b02d96';
export const SITES_USED_FOR_BENCHING = [
    [LANDING_PAGE_OID, "landing page"],
    [FLATLAND_OID, "flatland"],
    [FLATLANDER_OID, "flatlander"]
]
