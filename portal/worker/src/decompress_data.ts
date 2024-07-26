// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

/**
 * Decompresses the contents of the buffer according to the content encoding.
 */
export async function decompressData(
    data: ArrayBuffer,
    contentEncoding: string
): Promise<ArrayBuffer | null> {
    if (contentEncoding === "plaintext") {
        return data;
    }
    // check that contentencoding is a valid CompressionFormat
    if (["gzip", "deflate", "deflate-raw"].includes(contentEncoding)) {
        const enc = contentEncoding as CompressionFormat;
        const blob = new Blob([data], { type: "application/gzip" });
        const stream = blob.stream().pipeThrough(new DecompressionStream(enc));
        const response = await new Response(stream).arrayBuffer().catch((e) => {
            console.error("DecompressionStream error", e);
        });
        if (response) return response;
    }
    return null;
}
