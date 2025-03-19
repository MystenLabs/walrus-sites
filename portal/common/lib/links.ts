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
export function getObjectIdLink(url: URL): DomainDetails | null {
    logger.info({ message: "Trying to extract the sui link from:", originalUrl: url.href});
    const suiResult = /^https:\/\/(.+)\.suiobj.invalid\/(.*)$/.exec(url.href);
    if (suiResult) {
        const parsedDomainDetails = { subdomain: suiResult[1], path: "/" + suiResult[2] };
        logger.info({ message: "Matched sui link", parsedDomainDetails: parsedDomainDetails });
        return parsedDomainDetails;
    }
    return null;
}

/**
 * Checks if there is a link to a walrus resource in the path.
 *
 * These "Walrus Site links" have the following format:
 * `/[blobid.walrus]`
 */
export function getBlobIdLink(url: URL): string | null {
    logger.info({ message: "Trying to extract the walrus link from:", originalUrl: url.href });
    const walrusResult = /^https:\/\/blobid\.walrus.invalid\/(.+)$/.exec(url.href);
    if (walrusResult) {
        logger.info({ message: "Matched walrus link using blobid.walrus", walrusResult: walrusResult[1]});
        return walrusResult[1];
    }
    return null;
}
