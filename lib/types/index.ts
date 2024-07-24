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
    content_type: string;
    content_encoding: string;
    blob_id: string;
};
