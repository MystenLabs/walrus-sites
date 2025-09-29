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
			})
		},
		loggers: [
			{ category: "server-portal", lowestLevel: "debug", sinks: ["console"] }
		],
	});
	const tapeLogger = getLogger(["server-portal"]);

	// Integrate Tapelog by connecting the logger predicates to the tapeLogger instance
	logger.setInfoPredicate((...args: any[]) => {
		const message = args[0];
		tapeLogger.info(message, getPropertiesWithContext(...args), logger.context);
	});

	logger.setDebugPredicate((...args: any[]) => {
		const message = args[0];
		tapeLogger.debug(message, getPropertiesWithContext(...args), logger.context);
	});

	logger.setWarnPredicate((...args: any[]) => {
		const message = args[0];
		tapeLogger.warn(message, getPropertiesWithContext(...args), logger.context);
	});

	logger.setErrorPredicate((...args: any[]) => {
		// If args contains only one element, pass undefined as structured data
		const message = args[0];
		tapeLogger.error(message, getPropertiesWithContext(...args), logger.context);
	});
}

/**
 * Extracts structured properties from the provided arguments and attaches the current logging context.
 * If additional arguments are present, they are treated as structured data; otherwise, a default context is returned.
 *
 * @param args - The arguments passed to the logger, where the first argument is the message and subsequent arguments are optional structured data.
 * @returns An object containing structured properties and the current logging context.
 */
function getPropertiesWithContext(...args: any[]) {
	let properties: any = args.length > 1 ? args.slice(1) : undefined;
	if (properties) {
		properties.context = logger.context;
	} else {
		properties = { context: logger.context! };
	}
	return properties;
}
