// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { NextResponse } from 'next/server'
import { getDomain, getSubdomainAndPath } from '@lib/domain_parsing'
import { redirectToAggregatorUrlResponse, redirectToPortalURLResponse } from '@lib/redirects'
import { getBlobIdLink, getObjectIdLink } from '@lib/links'
import { resolveAndFetchPage } from '@lib/page_fetching'
import { removeLastSlash } from '@lib/domain_parsing'

export async function GET(req: Request) {
    const originalUrl = req.headers.get('x-original-url')
    if (!originalUrl) {
        throw new Error('No original url found in request headers')
    }
    const url = new URL(originalUrl ?? req.url)

    const objectIdPath = getObjectIdLink(url.toString())
    if (objectIdPath) {
        console.log(`Redirecting to portal url response: ${url.toString()} from ${objectIdPath}`)
        return redirectToPortalURLResponse(url, objectIdPath)
    }
    const walrusPath: string | null = getBlobIdLink(url.toString())
    if (walrusPath) {
        console.log(`Redirecting to aggregator url response: ${req.url} from ${objectIdPath}`)
        return redirectToAggregatorUrlResponse(url, walrusPath)
    }

    // Check if the request is for a site.
    const parsedUrl = getSubdomainAndPath(url)
    const portalDomain = getDomain(url)
    const requestDomain = getDomain(url)

    if (requestDomain == portalDomain && parsedUrl && parsedUrl.subdomain) {
        console.log('fetching from the service worker')
        return resolveAndFetchPage(parsedUrl)
    }

    const scopeString = new URL(req.url).origin
    const onBaseURL = removeLastSlash(req.url) === removeLastSlash(scopeString)
    const onBaseURLIndex = removeLastSlash(req.url) === removeLastSlash(scopeString) + 'index.html'
    // Handle the case in which we are at the root `BASE_URL`
    if ( onBaseURL || onBaseURLIndex ) {
        console.log('serving the landing page')
        return // TODO: serve the landing page from /public
    }

    const response = await fetch(req)
    return response
}
