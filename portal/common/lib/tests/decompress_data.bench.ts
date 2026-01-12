// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { expect, test, describe, bench } from "vitest";
import { decompressData } from "@lib/decompress_data";
import * as Pako from "pako";

const mockContent = "Hello, Walrus!";
const encoder = new TextEncoder();
const decoder = new TextDecoder();
const mockContentEncodedTo: Uint8Array = encoder.encode(mockContent);
const gzipped: Uint8Array = Pako.gzip(mockContent);
const deflated: Uint8Array = Pako.deflate(mockContent);


describe('decompressData', () => {
    bench('decompress plaintext encoding', async () => {
        await decompressData(mockContentEncodedTo, "plaintext");
    });

    bench('decompress gzip encoding', async () => {
        await decompressData(gzipped, "gzip");
    });

    bench('decompress deflate encoding', async () => {
        await decompressData(deflated, "deflate");
    });
});
