// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { parseDomain, ParseResultType } from "parse-domain";
import { UrlExtract, DomainDetails } from "./types/index";

/**
 * Returns the domain (e.g. "example.com") of the given URL.
 * @param orig_url The URL to extract the domain from. e.g. "https://example.com"
 * @returns The domain of the URL. e.g. "example.com"
 */
export function getDomain(url: URL): string | null {
    return splitUrl(url).domain;
}

/**
* Given a URL, returns the subdomain and path.
* @param url e.g. "https://subname.name.walrus.site/"
* @returns domain details e.g. { subdomain: "subname", path: "/index.html"}
*/
export function getSubdomainAndPath(url: URL): DomainDetails | null {
    return splitUrl(url).details;
}

/**
* Given a URL, returns the extracted parts of it.
* @param url e.g. "https://subname.name.walrus.site/"
* @returns extracted details of a url e.g.
    {domain: name.walrus.site,
    { subdomain: "subname", path: "/index.html"}}
*/
function splitUrl(url: URL): UrlExtract {
    const parsed = parseDomain(url.hostname);
    let domain: string | null = null;
    let subdomain: string | null = null;
    if (parsed.type === ParseResultType.Listed) {
        domain = domain = parsed.domain + "." + parsed.topLevelDomains.join(".")
        subdomain = parsed.subDomains.join(".")
    } else if (parsed.type === ParseResultType.Reserved) {
        domain = parsed.labels[parsed.labels.length - 1];
        subdomain = parsed.labels.slice(0, parsed.labels.length - 1).join('.');
    } else {
        return {
            domain: null,
            details: null
        }
    }

    return {
        domain,
        details: {
            subdomain,
            path: url.pathname == "/" ? "/index.html" : removeLastSlash(url.pathname)
        }
    } as UrlExtract;
}


/**
 * Removes the last forward-slash if present.
 * Resources on chain are stored as `/path/to/resource.extension` exclusively.
 * @param path The path to remove the last forward-slash from.
 * @returns The path without the last forward-slash.
 */
export function removeLastSlash(path: string): string {
    return path.endsWith("/") ? path.slice(0, -1) : path;
}
