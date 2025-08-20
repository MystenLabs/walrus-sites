// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { describe, it, expect } from 'vitest'
import { Range } from './types'
import { QuiltPatch } from './quilt'

describe('derive quilt patch id from range', () => {
	it('happy path', () => {
		const cases = [
			{
				patch_id: "jkZ5UWT2SPN46czhU-brz4JmQlc-bJlD14MiBzldouUBAQACAA",
				base_id: "jkZ5UWT2SPN46czhU-brz4JmQlc-bJlD14MiBzldouU"
			},
			{
				patch_id: "jkZ5UWT2SPN46czhU-brz4JmQlc-bJlD14MiBzldouUBAgADAA",
				base_id: "jkZ5UWT2SPN46czhU-brz4JmQlc-bJlD14MiBzldouU"
			}
		];

		for (const { patch_id, base_id } of cases) {
			const range = extract_range(patch_id);
			const patch = new QuiltPatch(
				base_id,
				range
			);

			expect(
				patch.derive_id()
			).toBe(patch_id);
		}
	})
})

/**
 * Helper function to create the expected values of the tests QuiltPatch.derive_id tests.
 */
function extract_range(patch_id: string) {
	const identifier_buffer = Buffer.from(patch_id, 'base64')
	const dv = new DataView(identifier_buffer.buffer, identifier_buffer.byteOffset)
	const range = {
		start: dv.getUint16(33, true),
		end: dv.getUint16(35, true),
	} as Range
	return range
}
