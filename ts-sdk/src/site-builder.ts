// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { Ed25519Keypair } from '@mysten/sui/keypairs/ed25519'
import { SuiClient } from '@mysten/sui/client'
import { walrus } from '@mysten/walrus'
import { getFullnodeUrl } from '@mysten/sui/client'
import { loadSitesConfig } from '@utils/file_io'
import type { Transaction } from '@mysten/sui/transactions'

export class SiteBuilder {
    keypair: Ed25519Keypair
    client: SuiClient

    // TODO: set a default config path.
    constructor(configPath: string) {
        this.keypair = Ed25519Keypair.fromSecretKey(process.env.SECRET_KEY!)
        const sitesConfig = loadSitesConfig(configPath)
        this.client = new SuiClient({
            network: sitesConfig.default_context,
            url: getFullnodeUrl(sitesConfig.default_context),
        }).$extend(
            walrus({
                packageConfig: {
                    systemObjectId:
                        '0x98ebc47370603fe81d9e15491b2f1443d619d1dab720d586e429ed233e1255c1',
                    stakingPoolId:
                        '0x20266a17b4f1a216727f3eef5772f8d486a9e3b5e319af80a5b75809c035561d',
                },
            })
        )
    }

    public async run(tx: Transaction, gasBudget = 1_000_000_000) {
        tx.setGasBudget(gasBudget)
        const { errors } = await this.client.signAndExecuteTransaction({
            transaction: tx,
            signer: this.keypair,
        })
        return errors
    }
}
