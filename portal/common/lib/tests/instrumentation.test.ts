// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { describe, it, expect } from "vitest";
import { InstrumentationFacade } from "@lib/instrumentation";

// Use a dedicated port to avoid collisions with other test files that also
// import the instrumentationFacade singleton (which binds to port 9184).
const TEST_PORT = 9185;
const facade = new InstrumentationFacade(TEST_PORT);

describe("Instrumentation metrics endpoint", () => {
    it("should serve metrics on the Prometheus endpoint", async () => {
        const response = await fetch(`http://localhost:${TEST_PORT}/metrics`);

        expect(response.ok).toBe(true);
        expect(response.status).toBe(200);

        const metricsText = await response.text();
        expect(metricsText).toContain("# HELP");
        expect(metricsText).toContain("# TYPE");
    });

    it("should record and serve aggregator time metric with siteId label", async () => {
        // Record a test metric (only siteId label — blobOrPatchId and path were
        // dropped to avoid cardinality explosion in Prometheus/Mimir).
        facade.recordAggregatorTime(150, "0xtest123");

        // Fetch metrics
        const response = await fetch(`http://localhost:${TEST_PORT}/metrics`);
        const metricsText = await response.text();

        // Validate metric name exists
        expect(metricsText).toContain("ws_aggregator_fetching_time");

        // Validate description (HELP line)
        expect(metricsText).toContain(
            "# HELP ws_aggregator_fetching_time Time spent fetching data from Walrus aggregator",
        );

        // Validate unit
        expect(metricsText).toContain("# UNIT ws_aggregator_fetching_time ms");

        // Validate type
        expect(metricsText).toContain("# TYPE ws_aggregator_fetching_time histogram");

        // Validate labels and count (1 measurement recorded)
        expect(metricsText).toContain('ws_aggregator_fetching_time_count{siteId="0xtest123"} 1');

        // Validate labels and sum (150ms recorded)
        expect(metricsText).toContain('ws_aggregator_fetching_time_sum{siteId="0xtest123"} 150');

        // Validate correct bucket distribution for 150ms measurement
        // Buckets below 150ms should be 0
        expect(metricsText).toContain(
            'ws_aggregator_fetching_time_bucket{siteId="0xtest123",le="100"} 0',
        );

        // First bucket that includes 150ms (le="250") should be 1
        expect(metricsText).toContain(
            'ws_aggregator_fetching_time_bucket{siteId="0xtest123",le="250"} 1',
        );

        // All higher buckets should also be 1 (cumulative)
        expect(metricsText).toContain(
            'ws_aggregator_fetching_time_bucket{siteId="0xtest123",le="500"} 1',
        );
    });
});
