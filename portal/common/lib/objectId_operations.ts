// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { fromB64, fromHEX, isValidSuiObjectId, isValidSuiAddress, toHEX } from "@mysten/sui/utils";
const baseX = require('base-x');

const BASE36 = "0123456789abcdefghijklmnopqrstuvwxyz";
const b36 = baseX(BASE36);


/**
 * Subdomain encoding and parsing.
 *
 * Use base36 instead of HEX to encode object ids in the subdomain, as the subdomain must be < 64
 * characters.  The encoding must be case insensitive.
 */
export function subdomainToObjectId(subdomain: string): string | null {
    const objectId = Base36ToHEX(subdomain.toLowerCase());
    console.log(
        "obtained object id: ",
        objectId,
        isValidSuiObjectId(objectId),
        isValidSuiAddress(objectId)
    );
    return isValidSuiObjectId(objectId) ? objectId : null;
}

export function HEXtoBase36(objectId: string): string {
    return b36.encode(fromHEX(objectId.slice(2))).toLowerCase();
}

export function Base36ToHEX(objectId: string): string {
    return "0x" + toHEX(b36.decode(objectId.toLowerCase()));
}
