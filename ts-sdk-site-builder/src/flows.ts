// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { SuiClient } from "@mysten/sui/client";
import * as site from "../contracts/sites/walrus_site/site";
import * as site_metadata from "../contracts/sites/walrus_site/metadata";
import { Transaction } from "@mysten/sui/transactions";
import { Ed25519Keypair } from "@mysten/sui/keypairs/ed25519";


// TODO attach resources, construct metadata etc later on
export async function publish_site_flow(suiClient: SuiClient) {
    const tx = new Transaction();
    const metadata = site_metadata.newMetadata({
        arguments: {
            link: "https://docs.wal.app",
            imageUrl:
                "https://artprojectsforkids.org/wp-content/uploads/2022/02/How-to-Draw-a-Walrus.jpg",
            description: "A test site.",
            projectUrl: "https://wal.app",
            creator: "ML",
        },
    });
    const site_object = site.newSite({
        arguments: [tx.pure.string("test site"), metadata],
    });

    const keypair = Ed25519Keypair.fromSecretKey(process.env.SECRET_KEY!);
    const res = tx.add(site_object);
    tx.transferObjects([res], keypair.getPublicKey().toSuiAddress());
    tx.setGasBudget(1_000_000_000);
    const { errors } = await suiClient.signAndExecuteTransaction({
        transaction: tx,
        signer: keypair,
    });
    return errors
}
