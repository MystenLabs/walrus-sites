// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { describe, expect, test } from 'vitest';
import { getObjectIdLink, getBlobIdLink } from '@lib/links';
import { DomainDetails } from '@lib/types';

const getObjectIdLinkTestCases: [string, DomainDetails | null][] = [
    ["https://example.suiobj.invalid/resource/path", { subdomain: "example", path: "/resource/path" }],
    ["https://another-example.suiobj.invalid/another/resource/path",
        { subdomain: "another-example", path: "/another/resource/path" }],
    ["https://invalidsite.com/something", null],
    ["https://example.suiobj.invalid/", { subdomain: "example", path: "/" }],
    ["https://example.suiobj.invalid", { subdomain: "example", path: "/" }],
    ["https://example.suiobj.invalid/resource", { subdomain: "example", path: "/resource" }],
];

describe('getObjectIdLink', () => {
    getObjectIdLinkTestCases.forEach(([input, expected]) => {
        test(`Extracting from ${input} should return
            ${expected ? JSON.stringify(expected) : 'null'}`, () => {
            	const url = new URL(input);
                const result = getObjectIdLink(url as URL);
                expect(result).toEqual(expected);
            });
    });
});

const getBlobIdLinkTestCases: [string, string | null][] = [
    ["https://blobid.walrus.invalid/blob-id-123", "blob-id-123"],
    ["https://blobid.walrus.invalid/another-blob-id", "another-blob-id"],
    ["https://invalidsite.com/something", null],
    ["https://blobid.walrus.invalid/", null],
    ["https://blobid.walrus.invalid", null],
    ["https://blobid.walrus.invalid/blob-id-456", "blob-id-456"],
];

describe('getBlobIdLink', () => {
    getBlobIdLinkTestCases.forEach(([input, expected]) => {
        test(`Extracting from ${input} should return ${expected ? expected : 'null'}`, () => {
			const url = new URL(input);
            const result = getBlobIdLink(url as URL);
            expect(result).toEqual(expected);
        });
    });
});
