// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { describe, expect, test } from 'vitest'
import { _parseDomain } from './domain_parsing'
import { vi } from 'vitest'

vi.mock('parse-domain', () => ({
	parseDomain: vi.fn(() => ({
		type: 'LISTED',
		domain: 'flatland',
		hostname: 'flatland.wal.app',
		topLevelDomains: ['wal.app'],
		labels: ['flatland', 'wal.app'],
		icann: {},
		subDomains: [''],
	})),
	fromUrl: vi.fn((url) => url),
	ParseResultType: { Listed: 'LISTED', Reserved: 'RESERVED' },
}));
import { splitUrl } from './domain_parsing'

describe('getDomain when "wal.app" is in parsed.topLevelDomains', () => {
	test('flatland.wal.app', () => {
		const input = 'https://flatland.wal.app/'
		const expected_portal_domain = 'wal.app'
		const expected_subdomain = 'flatland'
		const res = splitUrl(new URL(input) as URL)
		expect(res.domain).toEqual(expected_portal_domain)
		expect(res.details.subdomain).toEqual(expected_subdomain)
	})
})
