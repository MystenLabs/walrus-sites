// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0
import { SuiClient } from "@mysten/sui/client";
import { SITE_NAMES } from "./constants";

/**
 * Resolves the subdomain to an object ID using SuiNS.
 *
 * The subdomain `example` will look up `example.sui` and return the object ID if found.
 */
export async function resolveSuiNsAddress(
    client: SuiClient, subdomain: string
): Promise<string | null> {
    const suiObjectId: string = await client.call("suix_resolveNameServiceAddress", [
        subdomain + ".sui",
    ]);
    console.log("resolved suins name: ", subdomain, suiObjectId);
    return suiObjectId ? suiObjectId : null;
}

export function hardcodedSubdmains(subdomain: string): string | null {
    if (subdomain in SITE_NAMES) {
        return SITE_NAMES[subdomain];
    }
    return null;
}
