// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { base64UrlSafeEncode } from "@lib/url_safe_base64";

export class QuiltPatch {
	constructor(
		private quilt_blob_id: string,
		private quilt_patch_internal_id: string
	) { }

	/**
	* Derives the base64 URL-safe equivalent of the internal quilt patch ID.
	* This ID is created by combining the quilt blob ID with the internal patch identifier,
	* which includes the version, start index, and end index bytes.
	*/
	public derive_id(): string {
		const quilt_patch_internal_id = this.quilt_patch_internal_id.startsWith('0x')
			? this.quilt_patch_internal_id.slice(2)
			: this.quilt_patch_internal_id;

		// 1 version byte + 2 start index bytes + 2 index bytes
		const little_endian = true

		// Decode the quilt id to a buffer so that it is
		// also included in the base64 encoding.
		const buffer = Buffer.alloc(37);
		const blobIdBuffer = Buffer.from(this.quilt_blob_id, 'base64');
		blobIdBuffer.copy(buffer, 0, 0, Math.min(blobIdBuffer.length, 32));

		// Use a data view which makes it easier to work with endians.
		const view = new DataView(buffer.buffer, buffer.byteOffset);

		const quilt_patch_internal_id_buf = QuiltPatch.hexToBuffer(quilt_patch_internal_id)
		const internal_identifier_dv = new DataView(quilt_patch_internal_id_buf)
		const version = internal_identifier_dv.getInt8(0)
		const start_index = internal_identifier_dv.getInt16(1, little_endian)
		const end_index = internal_identifier_dv.getInt16(3, little_endian)

		// Include to the buffer the version, start and end index bytes.
		const version_offset = 32
		view.setUint8(version_offset, version);
		const start_index_offset = 33
		view.setUint16(start_index_offset, start_index, little_endian);
		const end_index_offset = 35
		view.setUint16(end_index_offset, end_index, little_endian);

		// Finally convert to base64.
		const base64String = base64UrlSafeEncode(new Uint8Array(buffer))

		// Some times there is padding `=` added to base64
		// Make sure that we do not surpass the 50 characters of the quilt id
		// with accidental padding.
		return base64String.slice(0, 50)
	}

	/**
	* Converts a hexadecimal string to an ArrayBuffer.
	* @param hex - The hexadecimal string to convert.
	* @returns An ArrayBuffer representing the bytes of the input hex string.
	*/
	public static hexToBuffer(hex: string): ArrayBuffer {
		const bytes = new Uint8Array(hex.length / 2);
		for (let i = 0; i < hex.length; i += 2) {
			bytes[i / 2] = parseInt(hex.slice(i, i + 2), 16);
		}
		return bytes.buffer;
	}
}
