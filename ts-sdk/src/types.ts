// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0
import { type ClientWithExtensions, type Experimental_CoreClient } from "@mysten/sui/experimental";
import { WalrusClient } from "@mysten/walrus";

export type WalrusSitesCompatibleClient = ClientWithExtensions<{
	core: Experimental_CoreClient;
	walrus: WalrusClient;
}>;
