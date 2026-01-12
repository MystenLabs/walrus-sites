// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { Inflate } from 'pako';
import logger from '@lib/logger';

/**
* Decompresses the contents of the buffer according to the content encoding.
*/
export async function decompressData(
	data: Uint8Array,
	contentEncoding: string
): Promise<Uint8Array | null> {
	try {
		logger.info('Decompressing data', { dataSize: data.length, contentEncoding })
		if (contentEncoding === "plaintext") {
			return data;
		}

		const encodingIsSupported = ["gzip", "deflate", "deflate-raw"].includes(contentEncoding);
		if (!encodingIsSupported) logger.warn('Unsupported encoding.', { contentEncoding })
		if (encodingIsSupported) {
			let decompressed: Uint8Array;
			switch (contentEncoding) {
				case "gzip":
					decompressed = streamInflate(data);
					break;
				case "deflate":
					decompressed = streamInflate(data);
					break;
				case "deflate-raw":
					decompressed = streamInflate(data, true);
					break;
				default:
					return null; // Unsupported encoding
			}
			return decompressed;
		}
	} catch (e) {
		logger.error("Failed to decompress data", { error: e });
	}
	return null;
}

/**
* Inflates the provided data chunk-by-chunk.
* This is used instead of just `inflate` to avoid zip bombs.
* @param data - The data to decompress.
* @param raw - Is the encoding `deflate-raw`?
* @param maximumFileSize - The maximum size processable gzip file. Default: 50 MB.
* @param chunkSize - The size of each chunk that will be inflated.
* @returns The inflated data.
*/
export function streamInflate(
	data: Uint8Array, raw = false, maximumFileSize = 52_428_800, chunkSize = 65_536
): Uint8Array<ArrayBufferLike> {
	if (maximumFileSize < chunkSize) {
		throw new Error(
			`maximumFileSize should be greater than ${chunkSize}! Current ${maximumFileSize}.`
		)
	}
	const totalChunks = maximumFileSize / chunkSize
	if (data.length > maximumFileSize) {
		throw new Error(`File too large, maximum deflated size of ${maximumFileSize} exceeded.`)
	}
	const inflator = new Inflate({raw});
	let endReached = false;
	for (let i = 0; i < totalChunks; i += chunkSize) {
		if (endReached) {
			break;
		}
		const chunk = data.subarray(i, Math.min(i + chunkSize, data.length));
		const isLastChunk = i + chunkSize >= data.length;
		endReached = inflator.push(chunk, isLastChunk);
	}
	if (inflator.err) { throw new Error(inflator.err.toString()); }
	const result = inflator.result
	if (typeof result != 'string') {
		return inflator.result as Uint8Array<ArrayBufferLike>
	}
	throw new Error('File decompression failed! Is result a string?')
}
