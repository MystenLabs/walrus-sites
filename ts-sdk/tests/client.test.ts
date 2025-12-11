// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { expect, it, describe } from 'bun:test'
import { SuiClient } from '@mysten/sui/client'
import { walrus } from '@mysten/walrus'
import { walrusSites } from '@client'
import { Ed25519Keypair } from '@mysten/sui/keypairs/ed25519'

const NETWORK = 'testnet'
const TESTNET_RPC_URL = 'https://fullnode.testnet.sui.io:443'

describe('walrusSitesClientShouldBeInitialisable', () => {
    it('initialises the walrus sites client', () => {
        const client = new SuiClient({
            network: NETWORK,
            url: TESTNET_RPC_URL,
        })
        const extendedClientWithWalrus = client.$extend(
            walrus({
                uploadRelay: {
                    host: 'https://upload-relay.testnet.walrus.space',
                    sendTip: {
                        max: 1_000,
                    },
                },
                network: NETWORK,
            })
        )
        const extendedClientWithWalrusAndWalrusSites =
            extendedClientWithWalrus.$extend(walrusSites())
        expect(extendedClientWithWalrusAndWalrusSites.base.network).toEqual(NETWORK)
        expect(extendedClientWithWalrusAndWalrusSites.walrus).toBeDefined()
        expect(extendedClientWithWalrusAndWalrusSites.walrus_sites).toBeDefined()
    })

    // Note: To run this test you should specify a big timeout. Otherwise, the test
    // will fail with a timeout error. To run it use this command:
    // $ bun test --test-name-pattern publishesASmallSite --timeout 120000
    it('publishesASmallSite', async () => {
        // Setup client
        const client = new SuiClient({
            network: NETWORK,
            url: TESTNET_RPC_URL,
        })
        const extendedClientWithWalrus = client.$extend(walrus({ network: NETWORK }))
        const extendedClientWithWalrusAndWalrusSites =
            extendedClientWithWalrus.$extend(walrusSites())
        expect(extendedClientWithWalrusAndWalrusSites.base.network).toEqual(NETWORK)
        expect(extendedClientWithWalrusAndWalrusSites.walrus).toBeDefined()
        expect(extendedClientWithWalrusAndWalrusSites.walrus_sites).toBeDefined()

        // Prepare
        const files = [
            { path: 'file1.txt', contents: new TextEncoder().encode('example string') },
            { path: 'file2.txt', contents: new TextEncoder().encode('example string 2') },
        ]
        const keypair = Ed25519Keypair.fromSecretKey(process.env.TEST_SIGNER!)
        const siteOptions = {
            siteName: 'DefinitelyNotABuggyTestSite',
            owner: keypair.toSuiAddress(),
            metadata: {
                link: 'https://example.com',
                image_url: 'https://example.com/image.png',
                description: 'example',
                project_url: 'https://example.com',
                creator: 'Tester The Creator',
            },
        }
        // Execute
        await extendedClientWithWalrusAndWalrusSites.walrus_sites?.publish(
            {
                files,
                siteOptions,
            },
            keypair
        )
    })
})
