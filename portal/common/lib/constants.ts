// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

export const NETWORK = "testnet";
export const AGGREGATOR = "https://aggregator.walrus-testnet.walrus.space";
export const SITE_PACKAGE = "0xc5bebae319fc9d2a9dc858b7484cdbd6ef219decf4662dc81a11dc69bb7a5fa7";
export const MAX_REDIRECT_DEPTH = 3;
export const SITE_NAMES: { [key: string]: string } = {
    // Any hardcoded (non suins) name -> object_id mappings go here
    // e.g.,
    // landing: "0x1234..."
};
// The default portal to redirect to if the browser does not support service workers.
export const FALLBACK_PORTAL = "blob.store";
// The string representing the ResourcePath struct in the walrus_site package.
export const RESOURCE_PATH_MOVE_TYPE = SITE_PACKAGE + "::site::ResourcePath";

export const TESTNET_RPC_LIST = [
    'https://fullnode.testnet.sui.io:443',
    'https://sui-testnet.public.blastapi.io/',
    'https://sui-testnet-endpoint.blockvision.org/',
    'https://rpc.ankr.com/sui_testnet',
    'https://sui.blockpi.network/v1/rpc/public'
];
