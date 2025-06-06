// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import {
    fromHex,
    isValidSuiObjectId,
    toHex
} from "@mysten/sui/utils";
import logger from "./logger";

const baseX = require('base-x');

const BASE36 = "0123456789abcdefghijklmnopqrstuvwxyz";
const b36 = baseX(BASE36);


/**
 * In case of base36 subdomain support, transform the subdomain from b36 to hex.
 *
 * Use base36 instead of HEX to encode object ids in the subdomain, as the subdomain must be < 64
 * characters.  The encoding must be case insensitive.
 */
export function subdomainToObjectId(subdomain: string): string | null {
    try{
        logger.info(
        	"Verifying whether the provided Walrus Site subdomain can be transformed from base36 to Hex",
         	{ subdomain }
        )
        const objectId = Base36toHex(subdomain.toLowerCase());
        return isValidSuiObjectId(objectId) ? objectId : null;
    } catch (e) {
        return null;
    }
}

export function HEXtoBase36(objectId: string): string {
    return b36.encode(fromHex(objectId.slice(2))).toLowerCase();
}

export function Base36toHex(objectId: string): string {
    return "0x" + toHex(b36.decode(objectId.toLowerCase()));
}
