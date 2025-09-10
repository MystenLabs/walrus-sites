// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { describe, expect, test } from 'vitest'
import { vi, beforeEach } from 'vitest'
import { splitUrl } from '@lib/domain_parsing'

/**
 * Make sure that splitURL supports both cases where `wal.app` is both included and not included
 * in the public suffix list.
 * For more details: https://linear.app/mysten-labs/issue/SEW-201/update-domain-parsingts-to-support-the-walrus-site-domain-as-suffix#comment-c8e8bbdd
 */
describe('splitUrl: mocking parse-domain output for "flatland.wal.app" in getDomain', () => {
	beforeEach(() => {
		vi.clearAllMocks();
	})

	test('wal.app is included in the topLevelDomains of the parseDomain output', () => {
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
		const input = 'https://flatland.wal.app/'
		const expected_portal_domain = 'wal.app'
		const expected_subdomain = 'flatland'
		const res = splitUrl(new URL(input) as URL)
		expect(res.domain).toEqual(expected_portal_domain)
		expect(res.details.subdomain).toEqual(expected_subdomain)
	})

	test('wal.app is not included in the topLevelDomains - but only "app" is included instead', () => {
		vi.mock('parse-domain', () => ({
			parseDomain: vi.fn(() => ({
				type: 'LISTED',
				domain: 'wal',
				hostname: 'flatland.wal.app',
				topLevelDomains: ['app'],
				labels: ['flatland', 'wal', 'app'],
				icann: {},
				subDomains: ['flatland'],
			})),
			fromUrl: vi.fn((url) => url),
			ParseResultType: { Listed: 'LISTED', Reserved: 'RESERVED' },
		}));
		const input = 'https://flatland.wal.app/'
		const expected_portal_domain = 'wal.app'
		const expected_subdomain = 'flatland'
		const res = splitUrl(new URL(input) as URL)
		expect(res.domain).toEqual(expected_portal_domain)
		expect(res.details.subdomain).toEqual(expected_subdomain)
	})
})
