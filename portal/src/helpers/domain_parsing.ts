// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { parseDomain, ParseResultType } from "parse-domain";
import { Path } from "../types/index";

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

export function getSubdomainAndPath(url: URL): Path | null {
    // At the moment we only support one subdomain level.
    const hostname = url.hostname.split(".");

    // TODO(giac): This should be changed to allow for SuiNS subdomains.
    if (hostname.length === 3 || (hostname.length === 2 && hostname[1] === "localhost")) {
        // Accept only one level of subdomain eg `subdomain.example.com` or `subdomain.localhost` in
        // case of local development.
        const path = url.pathname == "/" ? "/index.html" : removeLastSlash(url.pathname);
        return { subdomain: hostname[0], path } as Path;
    }
    return null;
}

/**
 * Removes the last forward-slash if present
 *
 * Resources on chain are stored as `/path/to/resource.extension` exclusively.
 */
function removeLastSlash(path: string): string {
    return path.endsWith("/") ? path.slice(0, -1) : path;
}
