// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { Hono } from 'hono'
import { getDomain, getSubdomainAndPath } from '@lib/domain_parsing'
import { redirectToAggregatorUrlResponse, redirectToPortalURLResponse } from '@lib/redirects'
import { getBlobIdLink, getObjectIdLink } from '@lib/links'
import { resolveAndFetchPage } from '@lib/page_fetching'
import { landingPage } from './landingPage'

const app = new Hono()

app.get('*', async (c) => {
    const url = new URL(c.req.url)
    const {subdomain, path} = getSubdomainAndPath(url) || {subdomain: null, path: null}
    const isAtWalrusSitesIndex = (path === '/' || path == '/index.html') && !(!!subdomain)
    if (isAtWalrusSitesIndex) {
        return c.html(
            landingPage
        )
    }

    const objectIdPath = getObjectIdLink(url.toString())
    if (objectIdPath) {
        console.log(`Redirecting to portal url response: ${url.toString()} from ${objectIdPath}`)
        return redirectToPortalURLResponse(url, objectIdPath)
    }
    const walrusPath: string | null = getBlobIdLink(url.toString())
    if (walrusPath) {
        console.log(`Redirecting to aggregator url response: ${c.req.url} from ${objectIdPath}`)
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

    return c.text('Not found', 404) // TODO render a 404 template page
})

export default app
