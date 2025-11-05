// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

// TODO: instead of this `index.ts` file, use the cli tool.

import { SuiClient } from "@mysten/sui/client";
import { publish_site } from "./flows/flows";

const suiClient = new SuiClient({
    network: "testnet",
    url: "https://fullnode.testnet.sui.io",
});

const errors = await publish_site(suiClient);
console.log(errors ? "Error!" : "Ok.");
