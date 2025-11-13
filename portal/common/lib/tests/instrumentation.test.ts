// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { describe, it, expect } from 'vitest';
import { instrumentationFacade } from '@lib/instrumentation';

describe('Instrumentation metrics endpoint', () => {
    it('should serve metrics on the Prometheus endpoint', async () => {
        const port = parseInt(process.env.PROMETHEUS_EXPORTER_PORT!) || 9184;
        const response = await fetch(`http://localhost:${port}/metrics`);

        expect(response.ok).toBe(true);
        expect(response.status).toBe(200);

        const metricsText = await response.text();
        expect(metricsText).toContain('# HELP');
        expect(metricsText).toContain('# TYPE');
    });

    it('should record and serve aggregator time metric with complete data', async () => {
        // Record a test metric
        instrumentationFacade.recordAggregatorTime(150, {
            siteId: '0xtest123',
            path: '/test.html',
            blobOrPatchId: 'blob456'
        });

        // Fetch metrics
        const port = parseInt(process.env.PROMETHEUS_EXPORTER_PORT!) || 9184;
        const response = await fetch(`http://localhost:${port}/metrics`);
        const metricsText = await response.text();

        // Validate metric name exists
        expect(metricsText).toContain('ws_aggregator_fetching_time');

        // Validate description (HELP line)
        expect(metricsText).toContain('# HELP ws_aggregator_fetching_time Time spent fetching data from Walrus aggregator');

        // Validate unit
        expect(metricsText).toContain('# UNIT ws_aggregator_fetching_time ms');

        // Validate type
        expect(metricsText).toContain('# TYPE ws_aggregator_fetching_time histogram');

        // Validate labels and count (1 measurement recorded)
        expect(metricsText).toContain('ws_aggregator_fetching_time_count{siteId="0xtest123",path="/test.html",blobOrPatchId="blob456"} 1');

        // Validate labels and sum (150ms recorded)
        expect(metricsText).toContain('ws_aggregator_fetching_time_sum{siteId="0xtest123",path="/test.html",blobOrPatchId="blob456"} 150');

        // Validate correct bucket distribution for 150ms measurement
        // Buckets below 150ms should be 0
        expect(metricsText).toContain('ws_aggregator_fetching_time_bucket{siteId="0xtest123",path="/test.html",blobOrPatchId="blob456",le="100"} 0');

        // First bucket that includes 150ms (le="250") should be 1
        expect(metricsText).toContain('ws_aggregator_fetching_time_bucket{siteId="0xtest123",path="/test.html",blobOrPatchId="blob456",le="250"} 1');

        // All higher buckets should also be 1 (cumulative)
        expect(metricsText).toContain('ws_aggregator_fetching_time_bucket{siteId="0xtest123",path="/test.html",blobOrPatchId="blob456",le="500"} 1');
    });
});
