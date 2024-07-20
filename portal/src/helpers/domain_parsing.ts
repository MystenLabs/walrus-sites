// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { parseDomain, ParseResultType } from "parse-domain";

/**
 * Returns the domain (e.g. "example.com") of the given URL.
 * @param orig_url The URL to extract the domain from. e.g. "https://example.com"
 * @returns The domain of the URL. e.g. "example.com"
 */
export function getDomain(orig_url: string): string {
    const url = new URL(orig_url);
    const urlWithoutProtocol = url.origin.replace(/^https?:\/\//, '');
    const urlWithoutPort = urlWithoutProtocol.replace(/:\d+$/, '');
    const parsed = parseDomain(urlWithoutPort);
    if (parsed.type === ParseResultType.Listed) {
        const domain = parsed.domain + "." + parsed.topLevelDomains.join(".");
        return domain;
    } else if (parsed.type === ParseResultType.Reserved) {
        return parsed.labels[parsed.labels.length - 1];
    } else {
        console.error("Error while parsing domain name:", parsed);
        throw new Error("Error while parsing domain name");
    }
}
