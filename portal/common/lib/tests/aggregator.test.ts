// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { describe, it, expect } from 'vitest';
import { blobAggregatorEndpoint, quiltAggregatorEndpoint } from '@lib/aggregator';

describe('blobAggregatorEndpoint', () => {
    it('builds the correct URL without trailing slash on base', () => {
        const url = blobAggregatorEndpoint('abc123', 'https://agg.example.com');
        expect(url).toBeInstanceOf(URL);
        expect(url.toString()).toBe('https://agg.example.com/v1/blobs/abc123');
    });

    it('builds the correct URL with trailing slash on base', () => {
        const url = blobAggregatorEndpoint('abc123', 'https://agg.example.com/');
        expect(url).toBeInstanceOf(URL);
        expect(url.toString()).toBe('https://agg.example.com/v1/blobs/abc123');
    });

    it('URL-encodes the blob_id', () => {
        const blobId = 'a/b c?d=e&f';
        const url = blobAggregatorEndpoint(blobId, 'https://agg.example.com');
        expect(url.toString()).toBe(
            `https://agg.example.com/v1/blobs/${encodeURIComponent(blobId)}`
        );
    });
});

describe('quiltAggregatorEndpoint', () => {
    it('builds the correct URL without trailing slash on base', () => {
        const url = quiltAggregatorEndpoint('patch-001', 'https://agg.example.com');
        expect(url).toBeInstanceOf(URL);
        expect(url.toString()).toBe(
            'https://agg.example.com/v1/blobs/by-quilt-patch-id/patch-001'
        );
    });

    it('builds the correct URL with trailing slash on base', () => {
        const url = quiltAggregatorEndpoint('patch-001', 'https://agg.example.com/');
        expect(url).toBeInstanceOf(URL);
        expect(url.toString()).toBe(
            'https://agg.example.com/v1/blobs/by-quilt-patch-id/patch-001'
        );
    });

    it('URL-encodes the quilt_patch_id', () => {
        const patchId = 'patch id/with?special=chars&and spaces';
        const url = quiltAggregatorEndpoint(patchId, 'https://agg.example.com');
        expect(url.toString()).toBe(
            `https://agg.example.com/v1/blobs/by-quilt-patch-id/${encodeURIComponent(patchId)}`
        );
    });
});
