// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { expect, it, describe, beforeEach } from 'bun:test'
import { SuiClient } from '@mysten/sui/client'
import { Transaction, SerialTransactionExecutor } from '@mysten/sui/transactions'
import { type ClientWithExtensions } from '@mysten/sui/experimental'
import { walrus, WalrusClient } from '@mysten/walrus'
import { walrusSites, WalrusSitesClient } from '@client'
import { Ed25519Keypair } from '@mysten/sui/keypairs/ed25519'

const NETWORK = 'testnet'
const PACKAGE_ADDRESS = '0xf99aee9f21493e1590e7e5a9aea6f343a1f381031a04a732724871fc294be799'
const EPOCHS = 2
const TESTNET_RPC_URL = 'https://fullnode.testnet.sui.io:443'
const KEYPAIR = Ed25519Keypair.fromSecretKey(process.env.TEST_SIGNER!)

describe('walrusClientTests', () => {
    let client: ClientWithExtensions<
        { [x: string]: WalrusSitesClient },
        ClientWithExtensions<{ walrus: WalrusClient }, SuiClient>
    >
    let executor: SerialTransactionExecutor

    beforeEach(() => {
        const baseClient = new SuiClient({
            network: NETWORK,
            url: TESTNET_RPC_URL,
            mvr: {
                overrides: {
                    packages: {
                        '@walrus/sites': PACKAGE_ADDRESS,
                    },
                },
            },
        })
        const extendedClientWithWalrus = baseClient.$extend(
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
        client = extendedClientWithWalrus.$extend(walrusSites())
        executor = new SerialTransactionExecutor({
            client,
            signer: KEYPAIR,
        })
    })

    it('initialisesTheWalrusSitesClient', () => {
        expect(client.base.network).toEqual(NETWORK)
        expect(client.walrus).toBeDefined()
        expect(client.walrus_sites).toBeDefined()
    })

    // This test would be much shorter if we use
    // `client.tx.createAndAddResource` but I expanded its' internal
    // steps here in order to debug it. For some reason I get the following
    // error a ValiError: Invalid input: Received "0x000000000000000000000000000000000000000000000000000@walrus/sites"
    it('createNewResourcePTB', async () => {
        const tx = new Transaction()
        tx.setSender(KEYPAIR.toSuiAddress())
        const range = client.walrus_sites?.call.newRange({
            arguments: {
                rangeStart: 0,
                rangeEnd: 1,
            },
        })!
        tx.add(range)
        const metadata = client.walrus_sites?.call.newMetadata({
            arguments: {
                link: 'https://example.com',
                imageUrl: 'https://example.com/image.png',
                description: 'Example description',
                projectUrl: 'https://example.com/project',
                creator: 'Example Creator',
            },
        })!
        tx.add(metadata)
        const site = client.walrus_sites?.call.newSite({
            arguments: {
                name: 'Example Site',
                metadata,
            },
        })!
        tx.add(site)
        tx.transferObjects([site], KEYPAIR.toSuiAddress()) // <- ValiError: Invalid input: Received "0x000000000000000000000000000000000000000000000000000@walrus/sites"
        const resource = client.walrus_sites?.call.newResource({
            arguments: {
                path: 'path',
                blobId: 123n,
                blobHash: 3219229n,
                range,
            },
        })!
        const addResource = client.walrus_sites?.call.addResource({
            arguments: { site, resource },
        })!
        tx.add(addResource) // <- ValiError: Invalid input: Received "0x000000000000000000000000000000000000000000000000000@walrus/sites"
        tx.setGasBudget(1000000000)

        const res = await KEYPAIR.signAndExecuteTransaction({ transaction: tx, client })

        console.log(res.digest)
    })

    // Note: To run this test you should specify a big timeout. Otherwise, the test
    // will fail with a timeout error. To run it use this command:
    // $ bun test --test-name-pattern publishesASmallSite --timeout 120000
    it.skip('publishesASmallSite', async () => {
        // Prepare
        const files = [
            { path: 'file1.txt', contents: new TextEncoder().encode('<div>AAAFirst</div>') },
            { path: 'file2.txt', contents: new TextEncoder().encode('<div>BBBSecond</div>') },
        ]
        const siteOptions = {
            siteName: 'DefinitelyNotABuggyTestSite',
            owner: KEYPAIR.toSuiAddress(),
            siteMetadata: {
                link: 'https://example.com',
                image_url: 'https://example.com/image.png',
                description: 'example',
                project_url: 'https://example.com',
                creator: 'Tester The Creator',
            },
        }
        // Execute
        let res = await client.walrus_sites?.publish(
            {
                files,
                siteOptions,
            },
            EPOCHS,
            KEYPAIR
        )
        console.log(res)
    })
})
