// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import logger from "@lib/logger";
import { configure, getConsoleSink, getJsonLinesFormatter } from "@logtape/logtape";
import { getLogger } from "@logtape/logtape";

/**
 * Configures the Tapelog logging library for JSON line formatting and logging categories,
 * integrating it with the existing logger.
 */
export async function setupTapelog() {
	// Tapelog configuration
	await configure({
		sinks: {
			console: getConsoleSink({
			formatter: getJsonLinesFormatter(),
		}) },
		loggers: [
			{ category: "server-portal", lowestLevel: "debug", sinks: ["console"] }
		],
	});
	const tapeLogger = getLogger(["server-portal", "my-module"]);

	// Integrate Tapelog by connecting the logger predicates to the tapeLogger instance
	logger.setInfoPredicate((args) => tapeLogger.info(args));
	logger.setDebugPredicate((args) => tapeLogger.debug(args));
	logger.setWarnPredicate((args) => tapeLogger.warn(args));
	logger.setErrorPredicate((args) => tapeLogger.error(args));
}
