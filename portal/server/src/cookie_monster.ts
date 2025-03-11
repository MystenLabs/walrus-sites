// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import * as cookie from 'cookie'

export class CookieMonster {
	/**
	* Eats existing cookies, and prevents new cookies from being
	* included in the request. Nom nom nom!
	* @param request - The incoming request for a Site resource.
	* @param response - The response of a Site resource.
	*/
	static eatCookies(request: Request, response: Response) {
		CookieMonster.eatExistingCookies(request, response)
		CookieMonster.eatNewCookies(request)
		return
	}

	private static eatExistingCookies(request: Request, response: Response) {
		// If there are existing cookies, eat (delete) them.
		const cookieHeader = request.headers.get('Cookie');
		let cookies: Record<string, string | undefined>;
		if (cookieHeader) {
			cookies = cookie.parse(cookieHeader);
			// For each cookie, set it to expire in the past
			Object.keys(cookies).forEach(name => {
				response.headers.set('Set-Cookie',
					cookie.serialize(name, '', {
						expires: new Date(0), // Set to epoch time
						path: '/' // Match the cookie path
					})
				);
			});
		}
	}

	private static eatNewCookies(request: Request) {
		// If a request tries to set a new cookie, do not permit it.
		const setCookieHeader = request.headers.get('Set-Cookie');
		if (setCookieHeader) {
			// Remove Set-Cookie header from the request
			request.headers.delete('Set-Cookie');
		}
	}
}

export default CookieMonster;
