// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { expect, test, describe } from "vitest";
import { decompressData, streamInflate } from "@lib/decompress_data";
import * as Pako from "pako";

const mockContent = "Hello, Walrus!";
const encoder = new TextEncoder();
const decoder = new TextDecoder();
const mockContentEncodedTo: Uint8Array = encoder.encode(mockContent);

describe('decompressData', () => {
    test('decompress plaintext encoding', async () => {
        const result: Uint8Array = await decompressData(
            mockContentEncodedTo,
            "plaintext"
        );
        expect(result).toEqual(mockContentEncodedTo);
        expect(mockContent).toEqual(decoder.decode(result));
    });

    test('decompress gzip encoding', async () => {
        const gzipped: Uint8Array = Pako.gzip(mockContent);
        const result = await decompressData(gzipped, "gzip");
        expect(result).toEqual(mockContentEncodedTo);
        expect(mockContent).toEqual(decoder.decode(result));
    });

    test('decompress deflate encoding', async () => {
        const deflated: Uint8Array = Pako.deflate(mockContent);
        const result = await decompressData(deflated, "deflate");
        expect(result).toEqual(mockContentEncodedTo);
        expect(mockContent).toEqual(decoder.decode(result));
    });

    test('decompress unsupported encoding', async () => {
        const result = await decompressData(mockContentEncodedTo, "unsupported");
        expect(result).toBeNull();
    });
});

describe('streamInflate', () => {
	const testString = "Hello, Walrus!";
	const encoder = new TextEncoder();
	const decoder = new TextDecoder();
	const encoded = encoder.encode(testString);

	test('inflates deflated data correctly', () => {
		const deflated = Pako.deflate(testString);
		const inflated = streamInflate(deflated);
		expect(decoder.decode(inflated)).toEqual(testString);
	});

	test('inflates raw deflated data correctly', () => {
		const deflatedRaw = Pako.deflateRaw(testString);
		const inflated = streamInflate(deflatedRaw, true);
		expect(decoder.decode(inflated)).toEqual(testString);
	});

	test('handles empty data', () => {
		const deflated = Pako.deflate("");
		const inflated = streamInflate(deflated);
		expect(decoder.decode(inflated)).toEqual("");
	});

	test('throws error for data exceeding maximum size', () => {
		const smallMaxSize = 1000;
		const largeData = new Uint8Array(smallMaxSize + 1);
		expect(() => streamInflate(largeData, false, smallMaxSize)).toThrow("maximumFileSize should be greater than 65536! Current 1000.");
	});

	test('throws error for invalid maximum file size', () => {
		const tooSmallMaxSize = 1000;
		expect(() => streamInflate(encoded, false, tooSmallMaxSize)).toThrow("maximumFileSize should be greater than");
	});

	test('handles larger realistic payloads', () => {
		const largerText = "A".repeat(200000);
		const deflated = Pako.deflate(largerText);
		const inflated = streamInflate(deflated);
		expect(decoder.decode(inflated)).toEqual(largerText);
	});

	test('handles different chunk sizes', () => {
        const largerText = "A".repeat(200000);
        const deflated = Pako.deflate(largerText);

        // Small chunks
        const inflatedSmallChunks = streamInflate(deflated, false, 52_428_800, 1024);
        expect(decoder.decode(inflatedSmallChunks)).toEqual(largerText);

        // Large chunks
        const inflatedLargeChunks = streamInflate(deflated, false, 52_428_800, 131072);
        expect(decoder.decode(inflatedLargeChunks)).toEqual(largerText);
    });

    test('handles chunk size near data length', () => {
        const text = "Test data for chunking";
        const deflated = Pako.deflate(text);

        // Chunk size exactly matches data length
        const inflatedExactChunk = streamInflate(deflated, false, 52_428_800, deflated.length);
        expect(decoder.decode(inflatedExactChunk)).toEqual(text);

        // Chunk size slightly smaller than data length
        const inflatedSmallerChunk = streamInflate(deflated, false, 52_428_800, 52_428_799);
        expect(decoder.decode(inflatedSmallerChunk)).toEqual(text);
    });
});
