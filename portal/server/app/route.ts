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
        console.log('serving the landing page from walrus sites')
        const blobId = '55onty23j6xl6axb7z2o03t5zs6gmosw30qjb4lqr3t60ukc0a'
        const landingPageURLString = `https://${blobId}.blob.store`
        const resourcePath = parsedUrl?.path == '/index.html' ?
            '/index-sw-enabled.html' :
            parsedUrl?.path ?? '/index-sw-enabled.html'
        console.log(`fetching ${landingPageURLString}${resourcePath}`)
        const response = await fetch(`${landingPageURLString}${resourcePath}`)
        const data = await response.text()

        // Proxy requests for CSS and fonts
        const proxiedData = data.replace(
            /(href|src)="\/([^"]+)"/g,
            `$1="${landingPageURLString}/$2"`
        )

        // Return the fetched data as a response
        return new Response(proxiedData, {
            headers: {
                'content-type': response.headers.get('content-type') ?? 'text/html'
            }
        })
    }

    return new Response(`Resource at ${originalUrl} not found!`, { status: 404 })
}
