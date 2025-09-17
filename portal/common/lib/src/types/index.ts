// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0
import logger from "@lib/logger";

/**
 * The origin of the request, divided into subdomain and path.
 */
export type DomainDetails = {
    subdomain: string;
    path: string;
};

/**
 * The extracted parts contained in a URL.
 */
export type UrlExtract = {
    details: DomainDetails | null;
    domain: string | null;
};

/**
 * The metadata for a site resource, as stored on chain.
 */
export type Resource = {
    path: string;
    headers: Map<string, string>;
    blob_id: string;
    blob_hash: string;
    range: Range | null;
};

export type Range = {
    start: number | null;
    end: number | null;
};

/**
 * Checks if the range is well formed.
 */
function isRangeValid(range: Range): boolean {
    if (range.start == null && range.end == null) {
        return false;
    }
    if (range.start !== null && range.start < 0) {
        return false;
    }
    if (range.end !== null && range.end < 0) {
        return false;
    }
    if (range.start !== null && range.end !== null && range.start > range.end) {
        return false;
    }
    return true;
}

/**
 * Creates an HTTP range header from the given range.
 *
 * Checks if the range is valid and returns the range header.
 */
function rangeToHttpHeader(range: Range): string {
    if (!isRangeValid(range)) {
        throw new Error(`Invalid range: start=${range.start} end=${range.end}`);
    }
    return `bytes=${range.start ?? ""}-${range.end ?? ""}`;
}

export function optionalRangeToHeaders(range: Range | null): { [key: string]: string } {
    if (range) {
    	let headers = rangeToHttpHeader(range)
    	logger.info("Appending range headers", { headers });
        return { range: headers };
    } else {
        logger.warn("No range headers provided");
        return {};
    }
}

export type VersionedResource = Resource & {
    version: string; // the sui object version of the site resource
    objectId: string; // the sui object id of the site resource
};

/**
 * Type guard for the Resource type.
 */
export function isResource(obj: any): obj is Resource {
    return (
        obj &&
        typeof obj.path === "string" &&
        typeof obj.headers === "object" &&
        typeof obj.blob_id === "string" &&
        typeof obj.blob_hash === "string" &&
        typeof obj.range === "object"
    );
}

/**
 * Type guard for the VersionedResource type.
 */
export function isVersionedResource(resource: any): resource is VersionedResource {
    return (
        resource &&
        isResource(resource) &&
        typeof resource === "object" &&
        "version" in resource &&
        "objectId" in resource
    );
}

/**
 * Routes is an optional dynamic field object belonging to each site.
 */
export type Routes = {
    routes_list: Map<string, string>;
};

/**
 * Type guard for the Routes type.
 */
export function isRoutes(obj: any): obj is Routes {
    return obj && typeof obj.routes_list === "object" && obj.routes_list instanceof Map;
}

/**
 * A NameRecord entry of SuiNS Names.
 */
export type NameRecord = {
	name: string;
	nftId: string;
	targetAddress: string;
	expirationTimestampMs: number;
	data: Record<string, string>;
	avatar?: string;
	contentHash?: string;
	walrusSiteId?: string;
};

/**
 * The Sui client network type.
 */
export type Network = 'testnet' | 'mainnet';
