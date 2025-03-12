// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { NextResponse } from 'next/server'
import type { NextRequest } from 'next/server'
import CookieMonster from 'src/cookie_monster'

export function middleware(request: NextRequest, response: NextResponse) {
	response = NextResponse.next()
	CookieMonster.eatCookies(request, response)
    const urlOriginal = extractUrlFrom(request)
    const alreadyAtRoot = request.nextUrl.pathname === '/'
    // Bypass middleware for walrus-sites-sw.js
	if (request.nextUrl.pathname.endsWith('walrus-sites-sw.js') || request.nextUrl.pathname === '/api/healthz') {
		return response
	}
    if (alreadyAtRoot) {
        response.headers.set('x-original-url', urlOriginal)
        return response
    }
    const urlRedirect = new URL('/', request.url)
    response = NextResponse.rewrite(urlRedirect)
    response.headers.set('x-original-url', urlOriginal)
    return response
}

export const config = {
    matcher: '/(.*)',
}

function extractUrlFrom(request: NextRequest): string {
    const hostname = request.headers.get('x-forwarded-host') ?? request.headers.get('host')
    if (!hostname) {
        throw new Error('No hostname found in request header')
    }
    return `${request.nextUrl.protocol}//${hostname}${request.nextUrl.pathname}`
}
