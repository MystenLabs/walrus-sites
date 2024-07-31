// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { Hono } from 'hono'
import { getDomain, getSubdomainAndPath } from '@lib/domain_parsing'
import { redirectToAggregatorUrlResponse, redirectToPortalURLResponse } from '@lib/redirects'
import { getBlobIdLink, getObjectIdLink } from '@lib/links'
import { resolveAndFetchPage } from '@lib/page_fetching'
import { html } from 'hono/html'

const app = new Hono()

app.get('/', async (c) => {
    return c.html(
        html`
            <h1>Welcome to Hono!</h1>
            <p>
                This is a simple web application built with Hono.
            </p>
            <p>
                <a href="/about">About</a>
            </p>
        `
    )
})

app.get('*', async (c) => {
    const url = new URL(c.req.url)
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

    return c.text('Not found', 404)
})

export default app
