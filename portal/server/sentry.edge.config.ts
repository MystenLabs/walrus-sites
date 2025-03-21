// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

// This file configures the initialization of Sentry for edge features (middleware, edge routes, and so on).
// The config you add here will be used whenever one of the edge features is loaded.
// Note that this config is unrelated to the Vercel Edge Runtime and is also required when running locally.
// https://docs.sentry.io/platforms/javascript/guides/nextjs/

import * as Sentry from "@sentry/nextjs";
import { config } from "src/configuration_loader";

if (config.enableSentry) {
    Sentry.init({
        dsn: config.sentryDsn,

        // Define how likely traces are sampled. Adjust this value in production, or use tracesSampler for greater control.
        tracesSampleRate: config.sentryTracesSampleRate,

        // Setting this option to true will print useful information to the console while you're setting up Sentry.
        debug: false,
    });
}
