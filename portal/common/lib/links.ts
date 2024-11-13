// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { DomainDetails } from "./types";
import logger from "./logger";

/**
 * Checks if there is a link to a sui resource in the path.
 *
 * These "Walrus Site links" have the following format:
 * `/[suinsname.sui]/resource/path`
 *  This links to a walrus site on sui.
 */
export function getObjectIdLink(url: string): DomainDetails | null {
    logger.info("Trying to extract the sui link from:", url);
    const suiResult = /^https:\/\/(.+)\.suiobj\/(.*)$/.exec(url);
    if (suiResult) {
        logger.info("Matched sui link: ", suiResult[1], suiResult[2]);
        return { subdomain: suiResult[1], path: "/" + suiResult[2] };
    }
    return null;
}

/**
 * Checks if there is a link to a walrus resource in the path.
 *
 * These "Walrus Site links" have the following format:
 * `/[blobid.walrus]`
 */
export function getBlobIdLink(url: string): string | null {
    logger.info("Trying to extract the walrus link from:", url);
    const walrusResult = /^https:\/\/blobid\.walrus\/(.+)$/.exec(url);
    if (walrusResult) {
        logger.info("Matched walrus link: ", walrusResult[1]);
        return walrusResult[1];
    }
    return null;
}
