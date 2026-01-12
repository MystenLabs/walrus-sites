// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { describe, it, expect } from "vitest";
import { QuiltPatch } from "@lib/quilt";

describe("derive quilt patch id from internal identifier", () => {
	it("happy path", () => {
		const cases = [
			{
				patch_id: "jkZ5UWT2SPN46czhU-brz4JmQlc-bJlD14MiBzldouUBAQACAA",
				base_id: "jkZ5UWT2SPN46czhU-brz4JmQlc-bJlD14MiBzldouU",
			},
			{
				patch_id: "jkZ5UWT2SPN46czhU-brz4JmQlc-bJlD14MiBzldouUBAgADAA",
				base_id: "jkZ5UWT2SPN46czhU-brz4JmQlc-bJlD14MiBzldouU",
			},
			{
				patch_id: "zHQTGUBR_e_87X3UIJwBTZRngDUNLKXnzrf5j6dWNAMBBQAGAA",
				base_id: "zHQTGUBR_e_87X3UIJwBTZRngDUNLKXnzrf5j6dWNAM",
			},
		];

		for (const { patch_id, base_id } of cases) {
			// internal identifier is extracted from the custom header x-wal-quilt-patch-internal-id
			const internal_identifier = extract_internal_identifier(patch_id);
			const patch = new QuiltPatch(base_id, internal_identifier);
			expect(patch.derive_id()).toBe(patch_id);
		}
	});
});

describe("bufferToHex", () => {
	it("should convert ArrayBuffer to hex string", () => {
		const buf = new Uint8Array([1, 0, 255, 16]).buffer;
		expect(bufferToHex(buf)).toBe("0100ff10");
	});

	it("should handle empty buffer", () => {
		const buf = new Uint8Array([]).buffer;
		expect(bufferToHex(buf)).toBe("");
	});
});

describe("hexToBuffer", () => {
	it("should convert hex string to ArrayBuffer", () => {
		const hex = "0100ff10";
		const buf = QuiltPatch.hexToBuffer(hex);
		expect(Array.from(new Uint8Array(buf))).toEqual([1, 0, 255, 16]);
	});

	it("should convert hex string to ArrayBuffer real patch 1", () => {
		const hex = "0100010002";
		const buf = QuiltPatch.hexToBuffer(hex);
		expect(Array.from(new Uint8Array(buf))).toEqual([0x01, 0x00, 0x01, 0x0, 0x02]);
	});

	it("should convert hex string to ArrayBuffer real patch 2", () => {
		const hex = "0100010003";
		const buf = QuiltPatch.hexToBuffer(hex);
		expect(Array.from(new Uint8Array(buf))).toEqual([0x01, 0x00, 0x01, 0x0, 0x03]);
	});

	it("should handle empty hex string", () => {
		const buf = QuiltPatch.hexToBuffer("");
		expect(Array.from(new Uint8Array(buf))).toEqual([]);
	});
});

describe("extract_internal_identifier", () => {
	it("extracts internal id from patch id", () => {
		const cases = [
			{
				patch_id: "zHQTGUBR_e_87X3UIJwBTZRngDUNLKXnzrf5j6dWNAMBAQAEAA",
				internal_id: "0x0101000400",
			},
			{
				patch_id: "zHQTGUBR_e_87X3UIJwBTZRngDUNLKXnzrf5j6dWNAMBBAAFAA",
				internal_id: "0x0104000500",
			},
			{
				patch_id: "zHQTGUBR_e_87X3UIJwBTZRngDUNLKXnzrf5j6dWNAMBBQAGAA",
				internal_id: "0x0105000600",
			},
			{
				patch_id: "zHQTGUBR_e_87X3UIJwBTZRngDUNLKXnzrf5j6dWNAMBBgAHAA",
				internal_id: "0x0106000700",
			},
			{
				patch_id: "zHQTGUBR_e_87X3UIJwBTZRngDUNLKXnzrf5j6dWNAMBBwAIAA",
				internal_id: "0x0107000800",
			},
			{
				patch_id: "zHQTGUBR_e_87X3UIJwBTZRngDUNLKXnzrf5j6dWNAMBCAAJAA",
				internal_id: "0x0108000900",
			},
		];
		for (const { patch_id, internal_id } of cases) {
			expect(extract_internal_identifier(patch_id)).toBe(internal_id);
		}
	});
});

/**
 * Helper function to create the expected values of the tests QuiltPatch.derive_id tests.
 */
function extract_internal_identifier(patch_id: string): string {
	const identifier_buffer = Buffer.from(patch_id, "base64");
	const dv = new DataView(identifier_buffer.buffer, identifier_buffer.byteOffset);
	const internal_id_buf = Buffer.alloc(5);
	const dv_internal_id = new DataView(internal_id_buf.buffer, internal_id_buf.byteOffset);
	dv_internal_id.setUint8(0, dv.getUint16(32, true));
	dv_internal_id.setUint16(1, dv.getUint16(33, true), true);
	dv_internal_id.setUint16(3, dv.getUint16(35, true), true);
	const hex = bufferToHex(dv_internal_id.buffer);
	return "0x" + hex;
}

function bufferToHex(buffer: ArrayBuffer) {
	const uint8Array = new Uint8Array(buffer);
	return Array.from(uint8Array)
		.map((b) => b.toString(16).padStart(2, "0"))
		.join("");
}
