// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { test, expect } from 'bun:test';
import CookieMonster from './cookie_monster';

test('eatCookies sets expired Set-Cookie headers to delete existing cookies', () => {
	const request = new Request('https://example.wal.app/page', {
		headers: {
			'Cookie': 'session=abc123',
			'Host': 'example.wal.app',
		},
	});
	const response = new Response('OK');

	CookieMonster.eatCookies(request, response);

	const setCookieHeaders = response.headers.getSetCookie();
	expect(setCookieHeaders.length).toBeGreaterThan(0);

	// Verify the session cookie is being deleted (value='deleted', expired)
	const sessionHeader = setCookieHeaders.find(h => h.startsWith('session=deleted'));
	expect(sessionHeader).toBeDefined();

	const expiresMatch = sessionHeader!.match(/Expires=([^;]+)/i);
	expect(expiresMatch).not.toBeNull();
	expect(new Date(expiresMatch![1]).getTime()).toBeLessThan(Date.now());
});
