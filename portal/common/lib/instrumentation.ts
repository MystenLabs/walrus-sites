// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { MeterProvider } from "@opentelemetry/sdk-metrics"
import { PrometheusExporter } from "@opentelemetry/exporter-prometheus"
import { Attributes, Counter, Meter, Histogram } from "@opentelemetry/api"
import logger from "./logger"

/**
 * Prometheus' instrumentation manager for minting backend.
 */
export class InstrumentationFacade {
    private meter: Meter;

    private num_requests_made_counter: Counter;
    private num_generic_errors_counter: Counter;

    private routingHistogram: Histogram<Attributes>;
    private fetchRoutesDynamicFieldObjectHistogram: Histogram<Attributes>;
    private resolveSuiNsAddressHistogram: Histogram<Attributes>;
    private resolveDomainAndFetchUrlHistogram: Histogram<Attributes>;

    constructor (port: number) {
        // Create the Prometheus exporter
        const exporter = new PrometheusExporter({ port: port }, () => {
            logger.info(`Prometheus Exporter endpoint running on port ${port}`);
        });

        // Initialize the Meter provider
        const meterProvider = new MeterProvider({ readers: [exporter] });
        this.meter = meterProvider.getMeter("my-meter");

        this.num_requests_made_counter = this.meter.createCounter(
            "ws_num_requests_made_counter",
            {
                description: "Number of requests made",
            }
        );

        this.num_generic_errors_counter = this.meter.createCounter(
            "ws_num_generic_errors_counter",
            {
                description: "Total number of generic errors",
            }
        );

        this.routingHistogram = this.meter.createHistogram("ws_routing_time", {
            description: "Time spent in Routing",
            unit: "ms",
        });

        this.fetchRoutesDynamicFieldObjectHistogram = this.meter.createHistogram("ws_fetch_routes_dynamic_field_object_time", {
            description: "Time spent in Fetching Routes Dynamic Field Object",
            unit: "ms",
        });

        this.resolveSuiNsAddressHistogram = this.meter.createHistogram("ws_resolve_sui_ns_address_time", {
            description: "Time spent in Resolving SuiNS Address",
            unit: "ms",
        });

        this.resolveDomainAndFetchUrlHistogram = this.meter.createHistogram("ws_resolve_domain_and_fetch_url_time", {
            description: "Time spent in resolve domain and fetch Url",
            unit: "ms",
        });
    }

    public increaseRequestsMade(total: number, _requestId: string) {
        this.num_requests_made_counter.add(total);
    }

    public bumpGenericErrors() {
        this.num_generic_errors_counter.add(1);
    }

    public recordRoutingTime(time: number, siteObjectId: string) {
        this.routingHistogram.record(time, { siteObjectId });
    }

    public recordFetchRoutesDynamicFieldObjectTime(time: number, siteObjectId: string) {
        this.fetchRoutesDynamicFieldObjectHistogram.record(time, { siteObjectId });
    }

    public recordResolveSuiNsAddressTime(time: number, subdomain: string) {
        this.resolveSuiNsAddressHistogram.record(time, { subdomain });
    }

    public recordResolveDomainAndFetchUrlResponseTime(time: number, resolvedObjectId: string) {
        this.resolveDomainAndFetchUrlHistogram.record(time, { resolvedObjectId });
    }

}

const port = parseInt(process.env.PROMETHEUS_EXPORTER_PORT!) || 9184;
export const instrumentationFacade = new InstrumentationFacade(port);
