// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { getDomain, getSubdomainAndPath } from '@lib/domain_parsing'
import { redirectToAggregatorUrlResponse, redirectToPortalURLResponse } from '@lib/redirects'
import { getBlobIdLink, getObjectIdLink } from '@lib/links'
import { resolveAndFetchPage } from '@lib/page_fetching'

export async function GET(req: Request) {
    const originalUrl = req.headers.get('x-original-url')
    if (!originalUrl) {
        throw new Error('No original url found in request headers')
    }
    const url = new URL(originalUrl)

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

    const atBaseUrl = portalDomain == url.host.split(':')[0]
    if (atBaseUrl) {
        console.log('Serving the landing page from walrus...')
        const blobId = '55onty23j6xl6axb7z2o03t5zs6gmosw30qjb4lqr3t60ukc0a'
        const resourcePath = parsedUrl?.path == '/index.html' ?
            '/index-sw-enabled.html' :
            parsedUrl?.path ?? '/index-sw-enabled.html'
        const response = await resolveAndFetchPage(
            {
                subdomain: blobId,
                path: resourcePath
            }
        )
        return response
    }

    return new Response(`Resource at ${originalUrl} not found!`, { status: 404 })
}
