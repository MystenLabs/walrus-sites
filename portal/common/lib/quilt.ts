// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { Range } from "./types";
import { base64UrlSafeEncode } from "./url_safe_base64";

// This is also a constant at the Walrus core code.
export const QUILT_VERSION_BYTE = 0x1;

export class QuiltPatch {
	public quilt_blob_id: string
	public version: number
	public start_index: number
	public end_index: number

	constructor(quilt_blob_id: string, range: Range) {
		this.quilt_blob_id = quilt_blob_id;
		this.version = QUILT_VERSION_BYTE
		this.start_index = range.start
		this.end_index = range.end
	}

	/// Derive the base64 equivalent of the internal quilt patch id.
	public derive_id(): string {
		// 1 version byte + 2 start index bytes + 2 index bytes
		const little_endian = true

		// Decode the quilt id to a buffer so that it is
		// also included in the base64 encoding.
		const buffer = Buffer.alloc(37);
		const blobIdBuffer = Buffer.from(this.quilt_blob_id, 'base64');
		blobIdBuffer.copy(buffer, 0, 0, Math.min(blobIdBuffer.length, 32));

		// Use a data view which makes it easier to work with endians.
		const view = new DataView(buffer.buffer, buffer.byteOffset);

		// Include to the buffer the version, start and end index bytes.
		const version_offset = 32
		view.setUint8(version_offset, this.version);
		const start_index_offset = 33
		view.setUint16(start_index_offset, this.start_index, little_endian);
		const end_index_offset = 35
		view.setUint16(end_index_offset, this.end_index, little_endian);

		// Finally convert to base64.
		const base64String = base64UrlSafeEncode(new Uint8Array(buffer))

		// Some times there is padding `=` added to base64
		// Make sure that we do not surpass the 50 characters of the quilt id
		// with accidental padding.
		return base64String.slice(0, 50)
	}
}
