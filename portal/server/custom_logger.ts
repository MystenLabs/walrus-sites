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
	logger.setInfoPredicate((...args: any[]) =>
		// If args contains only one element, pass undefined as structured data
		tapeLogger.info(args[0], args.length > 1 ? args.slice(1) : undefined)
	);
	logger.setDebugPredicate((...args: any[]) =>
		// If args contains only one element, pass undefined as structured data
		tapeLogger.debug(args[0], args.length > 1 ? args.slice(1) : undefined)
	);
	logger.setWarnPredicate((...args: any[]) =>
		// If args contains only one element, pass undefined as structured data
		tapeLogger.warn(args[0], args.length > 1 ? args.slice(1) : undefined)
	);
	logger.setErrorPredicate((...args: any[]) =>
		// If args contains only one element, pass undefined as structured data
		tapeLogger.error(args[0], args.length > 1 ? args.slice(1) : undefined)
	);
}
