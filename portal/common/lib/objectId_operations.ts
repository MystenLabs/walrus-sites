// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import {
    fromHex,
    isValidSuiObjectId,
    isValidSuiAddress,
    toHex
} from "@mysten/sui/utils";
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
    try{
        const objectId = Base36toHex(subdomain.toLowerCase());
        console.log(
            "obtained object id: ",
            objectId,
            isValidSuiObjectId(objectId),
            isValidSuiAddress(objectId)
        );
        return isValidSuiObjectId(objectId) ? objectId : null;
    } catch (e) {
        console.log("error converting subdomain to object id: ", e);
        return null;
    }
}

export function HEXtoBase36(objectId: string): string {
    return b36.encode(fromHex(objectId.slice(2))).toLowerCase();
}

export function Base36toHex(objectId: string): string {
    return "0x" + toHex(b36.decode(objectId.toLowerCase()));
}
