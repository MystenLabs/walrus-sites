// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { NextRequest, NextResponse } from 'next/server'
import { track } from '@vercel/analytics/server'

export async function send_to_web_analytics(request: NextRequest) {
    // Extract various details from the request
    const trackingData = extract_tracking_data(request)

    // Track the event with comprehensive data
    await track('route-access', trackingData)
}

function extract_tracking_data(request: NextRequest): TrackingData {
    const geo = request.geo || {}

    return {
        originalUrl: request.headers.get('x-original-url') || 'Unknown User Agent',

        // Network Information
        ip: request.ip || 'Unknown IP',
        country: geo.country || 'Unknown',
        region: geo.region || 'Unknown',
        city: geo.city || 'Unknown',

        // Client Information
        userAgent: request.headers.get('user-agent') || 'Unknown User Agent',
        referer: request.headers.get('referer') || 'Direct',

        // Additional Context
        timestamp: new Date().toISOString(),
        protocol: request.nextUrl.protocol,
    }
}

type TrackingData = {
    originalUrl: string
    city: string

    ip: string
    country: string
    region: string

    userAgent: string
    referer: string

    timestamp: string
    protocol: string
}
