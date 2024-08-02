// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { ungzip, inflate, inflateRaw } from 'pako';

/**
* Decompresses the contents of the buffer according to the content encoding.
*/
export async function decompressData(
    data: Uint8Array,
    contentEncoding: string
): Promise<Uint8Array | null> {
    try {
        if (contentEncoding === "plaintext") {
            return data;
        }

        const encodingIsSupported = ["gzip", "deflate", "deflate-raw"].includes(contentEncoding);

        if (encodingIsSupported) {
            let decompressed: Uint8Array;

            switch (contentEncoding) {
                case "gzip":
                    decompressed = ungzip(data);
                    break;
                case "deflate":
                    decompressed = inflate(data);
                    break;
                case "deflate-raw":
                    decompressed = inflateRaw(data);
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
