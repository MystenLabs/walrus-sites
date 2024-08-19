// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { expect, test, describe } from "vitest";
import { decompressData } from "./decompress_data";
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
