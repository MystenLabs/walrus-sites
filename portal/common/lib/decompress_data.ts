// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { ungzip, inflate, inflateRaw } from 'pako';

/**
* Decompresses the contents of the buffer according to the content encoding.
*/
export async function decompressData(
    data: ArrayBuffer,
    contentEncoding: string
): Promise<Uint8Array | null> {
    try {
        if (contentEncoding === "plaintext") {
            return data as Uint8Array;
        }

        const encodingIsSupported = ["gzip", "deflate", "deflate-raw"].includes(contentEncoding);

        if (encodingIsSupported) {
            let decompressed: Uint8Array;
            const uint8ArrayData = new Uint8Array(data);

            switch (contentEncoding) {
                case "gzip":
                    decompressed = ungzip(uint8ArrayData);
                    break;
                case "deflate":
                    decompressed = inflate(uint8ArrayData);
                    break;
                case "deflate-raw":
                    decompressed = inflateRaw(uint8ArrayData);
                    break;
                default:
                    return null; // Unsupported encoding
            }

            return decompressed;
        }
    } catch (e) {
        console.error("Pako decompression error", e);
    }
    return null;
}
