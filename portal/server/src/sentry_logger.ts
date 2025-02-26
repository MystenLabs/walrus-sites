// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import logger from "@lib/logger";
import * as Sentry from "@sentry/bun";

function addLoggingArgsToSentry(args: { [key: string]: any }) {
    Object.entries(args).forEach(([key, value]) => {
        if (key !== "message") { // Skipping the 'message' key
        Sentry.setTag(key, value);
        }
    });
}

function integrateLoggerWithSentry() {
    logger.setErrorPredicate(args => {
   		console.error(JSON.stringify(args).replace('\n', ''))
    	addLoggingArgsToSentry(args);
        Sentry.captureException(new Error(args.message ))
    });
    logger.setWarnPredicate(args => {
  		console.warn(JSON.stringify(args).replace('\n', ''))
        addLoggingArgsToSentry(args);
        Sentry.addBreadcrumb({ message: args.message, data: args, level: 'warning' })
    } );
    logger.setInfoPredicate(args => {
  		console.info(JSON.stringify(args).replace('\n', ''))
        addLoggingArgsToSentry(args);
        Sentry.addBreadcrumb({ message: args.message, data: args, level: 'info'})
    } );
    logger.setDebugPredicate(args => {
  		console.debug(JSON.stringify(args).replace('\n', ''))
        addLoggingArgsToSentry(args);
        Sentry.addBreadcrumb({ message: args.message, data: args, level: 'debug' })
    });
}

export default integrateLoggerWithSentry;
