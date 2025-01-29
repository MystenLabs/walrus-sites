// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import logger from "@lib/logger";
import * as Sentry from "@sentry/node";

function addLoggingArgsToSentry(args: { [key: string]: any }) {
    Object.entries(args).forEach(([key, value]) => {
        if (key !== "message") { // Skipping the 'message' key
            console.log(`${key}: ${value}`)
            Sentry.setTag(key, value);
        }
    });
}

function integrateLoggerWithSentry() {
    logger.setErrorPredicate(args => {
        addLoggingArgsToSentry(args);
        Sentry.captureException(new Error(args.message ))
    });
    logger.setWarnPredicate(args => {
        addLoggingArgsToSentry(args);
        Sentry.addBreadcrumb({ message: args.message, data: args, level: 'warning' })
    } );
    logger.setInfoPredicate(args => {
        addLoggingArgsToSentry(args);
        Sentry.addBreadcrumb({ message: args.message, data: args, level: 'info'})
    } );
    logger.setDebugPredicate(args => {
        addLoggingArgsToSentry(args);
        Sentry.addBreadcrumb({ message: args.message, data: args, level: 'debug' })
    });
}

export default integrateLoggerWithSentry;
