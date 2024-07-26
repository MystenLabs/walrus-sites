// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { NextResponse } from 'next/server'
import type { NextRequest } from 'next/server'

export function middleware(request: NextRequest) {
    const urlRedirect = new URL('/', request.url)
    const response = NextResponse.rewrite(urlRedirect)

    const urlOriginal = extractUrlFrom(request)

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