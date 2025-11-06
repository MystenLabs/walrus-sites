// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import * as site from "../contracts/sites/walrus_site/site";
import * as site_metadata from "../contracts/sites/walrus_site/metadata";
import { Transaction } from "@mysten/sui/transactions";
import { SiteBuilder } from "./site-builder";

// TODO attach resources, construct metadata etc later on
export async function publish_site_flow(tx: Transaction, siteBuilder: SiteBuilder) {
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

    const res = tx.add(site_object);
    tx.transferObjects([res], siteBuilder.keypair.getPublicKey().toSuiAddress());
    tx.setGasBudget(1_000_000_000);
    const { errors } = await siteBuilder.client.signAndExecuteTransaction({
        transaction: tx,
        signer: siteBuilder.keypair,
    });
    return errors;
}
