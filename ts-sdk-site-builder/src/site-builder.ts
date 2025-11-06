// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { Ed25519Keypair } from "@mysten/sui/keypairs/ed25519";
import { Client } from "./client";
import { SuiClient } from "@mysten/sui/client";
import { walrus } from "@mysten/walrus";
import { getFullnodeUrl } from "@mysten/sui/client";
import { parseSitesConfig, type SitesConfig } from "../utils/sites_config_parser";
import { readFileSync } from "fs";
import { parse as parseYaml } from "yaml";
import { join } from "path";

export function loadSitesConfig(): SitesConfig {
    const configPath = join(process.cwd(), "..", "..", "sites-config.yaml");
    const fileContent = readFileSync(configPath, "utf-8");
    const yamlData = parseYaml(fileContent);
    return parseSitesConfig(yamlData);
}

export function deduceRpcUrl(network: "testnet" | "mainnet"): string {
    return network == "testnet"
        ? "https://fullnode.testnet.sui.io"
        : "https://fullnode.mainnet.sui.io";
}

class SiteBuilder {
    keypair: Ed25519Keypair;
    client: SuiClient;

    constructor() {
        this.keypair = Ed25519Keypair.fromSecretKey(process.env.SECRET_KEY!);
        const sitesConfig = loadSitesConfig();
        this.client = new SuiClient({
            network: sitesConfig.default_context,
            url: getFullnodeUrl(sitesConfig.default_context),
        }).$extend(
            walrus({
                packageConfig: {
                    systemObjectId:
                        "0x98ebc47370603fe81d9e15491b2f1443d619d1dab720d586e429ed233e1255c1",
                    stakingPoolId:
                        "0x20266a17b4f1a216727f3eef5772f8d486a9e3b5e319af80a5b75809c035561d",
                },
            }),
        );
    }
}
