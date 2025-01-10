// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

// This file configures the initialization of Sentry on the server.
// The config you add here will be used whenever the server handles a request.
// https://docs.sentry.io/platforms/javascript/guides/nextjs/

import * as Sentry from "@sentry/nextjs";
import { config } from "configuration_loader";

if (config.enableSentry) {
    Sentry.init({
        dsn: config.sentryDsn,

        // Define how likely traces are sampled. Adjust this value in production, or use tracesSampler for greater control.
        tracesSampleRate: config.sentryTracesSampleRate,

        // Setting this option to true will print useful information to the console while you're setting up Sentry.
        debug: false,
    });
}
