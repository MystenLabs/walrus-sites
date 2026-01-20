// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import * as cookie from 'cookie'
import psl from 'psl'
import logger from '@lib/logger'

export class CookieMonster {
	/**
	* Eats existing cookies, and prevents new cookies from being
	* included in the request. Nom nom nom!
	* @param request - The incoming request for a Site resource.
	* @param response - The response of a Site resource.
	*/
	static eatCookies(request: Request, response: Response) {
		CookieMonster.eatExistingCookies(request, response)
		return
	}

	/**
	* Deletes the existing cookies by adding an artificial expired date.
	* Also handles subdomains by setting cookies for both the full domain and parent domain.
	*/
	private static eatExistingCookies(request: Request, response: Response) {
		// If there are existing cookies, eat (delete) them.
		const cookieHeader = request.headers.get('Cookie');
		const host = request.headers.get('Host') || '';
		const fullDomain = host.split(':')[0];

		if (cookieHeader) {
			const cookies = cookie.parse(cookieHeader);
			console.log('Cookies:', cookies);
			// For each cookie, set it to expire in the past for both the full domain and parent domain
			Object.keys(cookies).forEach(name => {
				const parentDomains = CookieMonster.getCookieParentDomains(fullDomain);
				// Eat cookie for the full domain
				const opts: cookie.SerializeOptions = {
					expires: new Date(1),
					path: '/',
					httpOnly: true,
				}
				response.headers.append('Set-Cookie', cookie.serialize(name, 'deleted', fullDomain ? { expires: new Date(1), path: '/', httpOnly: true, domain: fullDomain } : opts));

				// Eat cookie for the parent domains
				for (let parentDomain of parentDomains) {
					console.log(`Clearing cookie ${name} for parentDomain', parentDomain`);
					opts.domain = parentDomain;
					response.headers.append('Set-Cookie', cookie.serialize(name, 'deleted', opts));
				}
			});
		}
	}

	/**
	* Gets the parent domain if it's not a public suffix.
	* For example: walrusadventures.wal.app -> wal.app
	*/
	private static getParentDomain(host: string): string | null {
		const parts = host.split('.');
		if (parts.length < 2) return null;

		// Remove the first part (subdomain) and join the rest
		const parentDomain = parts.slice(1).join('.');
		return parentDomain;
	}

	private static getCookieParentDomains(host: string): string[] {
		const parentDomains = [];
		let currentParentDomain = this.getParentDomain(host);
		while (currentParentDomain) {
			const parsedPsl = psl.parse(currentParentDomain);
			if ('error' in parsedPsl) {
				logger.warn(`Unexpected PSL parse error for domain "${currentParentDomain}"`, parsedPsl);
				break;
			}
			if (!parsedPsl.sld) {
				break;
			}
			logger.debug(`sld for ${currentParentDomain} is ${parsedPsl.sld}`);
			parentDomains.push(currentParentDomain);
			currentParentDomain = this.getParentDomain(currentParentDomain);
		}
		return parentDomains;
	}
}

export default CookieMonster;
