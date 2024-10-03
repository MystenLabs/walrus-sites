// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

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
}

/**
 * The metadata for a site resource, as stored on chain.
 */
export type Resource = {
    path: string;
    headers: Map<string, string>;
    blob_id: string;
    blob_hash: string;
};

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
        typeof obj.path === 'string' &&
        typeof obj.headers === 'object' &&
        typeof obj.blob_id === 'string' &&
        typeof obj.blob_hash === 'string'
    );
}

/**
* Type guard for the VersionedResource type.
*/
export function isVersionedResource(resource: any): resource is VersionedResource {
    return resource && isResource(resource)
        && typeof resource === 'object'
        && 'version' in resource
        && 'objectId' in resource;
}
