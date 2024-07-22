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
    const urlStripped = stripProtocolAndPort(orig_url);
    const parsed = parseDomain(urlStripped);
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

/**
* Given a URL, returns the subdomain and path.
* @param url e.g. "https://subname.name.walrus.site/"
* @returns Path object e.g. { subdomain: "subname.name", path: "/index.html"}
*/

export function getSubdomainAndPath(url: URL): Path | null {
    const urlStripped = stripProtocolAndPort(url.toString());
    const parsed = parseDomain(urlStripped);
    console.log('parsed', parsed)
    let path: Path | null = null;
    if (parsed.type === ParseResultType.Listed) {
        return {
          subdomain: parsed.subDomains.join("."),
          path: url.pathname == "/" ? "/index.html" : removeLastSlash(url.pathname)
        } as Path;
    } else if ( parsed.type === ParseResultType.Reserved) {
      return {
        subdomain: parsed.labels.slice(0, parsed.labels.length-1).join('.'),
        path: url.pathname == "/" ? "/index.html" : removeLastSlash(url.pathname)
      } as Path;
    }
    return null;
}

/**
 * Removes the last forward-slash if present.
 * Resources on chain are stored as `/path/to/resource.extension` exclusively.
 * @param path The path to remove the last forward-slash from.
 * @returns The path without the last forward-slash.
 */
function removeLastSlash(path: string): string {
    return path.endsWith("/") ? path.slice(0, -1) : path;
}

/**
* Removes the protocol and port from a URL.
* @param url e.g. "https://example.com:8080"
* @returns string e.g. "example.com"
*/
function stripProtocolAndPort(url: string): string {
    return removeLastSlash(
        url
          .replace(/^https?:\/\//, '')
          .replace(/:\d+/, '')
    );
}
