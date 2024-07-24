import * as pako from 'pako';

// TODO: test that this works!

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

    const compressedData = new Uint8Array(data);

    try {
        switch (contentEncoding) {
            case "gzip":
                return pako.ungzip(compressedData).buffer;
            case "deflate":
                return pako.inflate(compressedData).buffer;
            case "deflate-raw":
                return pako.inflateRaw(compressedData).buffer;
            default:
                return null;
        }
    } catch (e) {
        console.error("Decompression error", e);
        return null;
    }
}
