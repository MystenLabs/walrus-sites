// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { expect, it, describe } from 'bun:test'
import { SuiClient } from '@mysten/sui/client'
import { walrus } from '@mysten/walrus'
import { walrusSites } from '@client'

const NETWORK = "testnet"
const TESTNET_RPC_URL = "https://fullnode.testnet.sui.io:443"

describe('walrusSitesClientShouldBeInitialisable', () => {
    it('initialises the walrus sites client', () => {
        const client = new SuiClient({
            network: NETWORK,
            url: TESTNET_RPC_URL
        })
        const extendedClientWithWalrus = client.$extend(walrus({ network: NETWORK }))
        const extendedClientWithWalrusAndWalrusSites = extendedClientWithWalrus.$extend(walrusSites())
        expect(extendedClientWithWalrusAndWalrusSites.base.network).toEqual(NETWORK)
        expect(extendedClientWithWalrusAndWalrusSites.walrus).toBeDefined()
        expect(extendedClientWithWalrusAndWalrusSites.walrus_sites).toBeDefined()
    })
})
