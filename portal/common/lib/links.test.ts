// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { describe, expect, test } from 'vitest';
import { getObjectIdLink, getBlobIdLink } from './links';
import { DomainDetails } from './types';

const getObjectIdLinkTestCases: [string, DomainDetails | null][] = [
    ["https://example.suiobj/resource/path", { subdomain: "example", path: "/resource/path" }],
    ["https://another-example.suiobj/another/resource/path",
        { subdomain: "another-example", path: "/another/resource/path" }],
    ["https://invalidsite.com/something", null],
    ["https://example.suiobj/", { subdomain: "example", path: "/" }],
    ["https://example.suiobj", null],
    ["https://example.suiobj/resource", { subdomain: "example", path: "/resource" }],
];

describe('getObjectIdLink', () => {
    getObjectIdLinkTestCases.forEach(([input, expected]) => {
        test(`Extracting from ${input} should return
            ${expected ? JSON.stringify(expected) : 'null'}`, () => {
                const result = getObjectIdLink(input);
                expect(result).toEqual(expected);
            });
    });
});

const getBlobIdLinkTestCases: [string, string | null][] = [
    ["https://blobid.walrus/blob-id-123", "blob-id-123"],
    ["https://blobid.walrus/another-blob-id", "another-blob-id"],
    ["https://invalidsite.com/something", null],
    ["https://blobid.walrus/", null],
    ["https://blobid.walrus", null],
    ["https://blobid.walrus/blob-id-456", "blob-id-456"],
];

describe('getBlobIdLink', () => {
    getBlobIdLinkTestCases.forEach(([input, expected]) => {
        test(`Extracting from ${input} should return ${expected ? expected : 'null'}`, () => {
            const result = getBlobIdLink(input);
            expect(result).toEqual(expected);
        });
    });
});
