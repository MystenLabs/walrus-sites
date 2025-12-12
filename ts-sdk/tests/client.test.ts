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

    it('createNewResourcePTB', async () => {
        const tx = new Transaction()
        tx.setSender(KEYPAIR.toSuiAddress())
        const range = client.walrus_sites?.call.newRange({
            arguments: {
                rangeStart: 0,
                rangeEnd: 1,
            },
        })!
        const metadata = client.walrus_sites?.call.newMetadata({
            arguments: {
                link: 'https://example.com',
                imageUrl: 'https://example.com/image.png',
                description: 'Example description',
                projectUrl: 'https://example.com/project',
                creator: 'Example Creator',
            },
        })!
        const site = client.walrus_sites?.call.newSite({
            arguments: {
                name: 'Example Site',
                metadata,
            },
        })!
        client.walrus_sites?.tx.createAndAddResource(tx, {
            newRangeOptions: { arguments: { rangeStart: 0, rangeEnd: 1 } },
            newResourceArguments: {
                path: 'path',
                blobId: 123,
                blobHash: 3219229,
                range,
            },
            site,
            resourceHeaders: new Map([['Content-Type', 'text/html']]),
        })
        tx.setGasBudget(1000000000)
        executor.executeTransaction(tx)
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
