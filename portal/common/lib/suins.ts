// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0
import { SITE_NAMES } from "./constants";
import { RPCSelector } from "./rpc_selector";
import logger from "./logger";
import { NameRecord } from "./types";
import { instrumentationFacade } from "./instrumentation";

export class SuiNSResolver {
	constructor(private rpcSelector: RPCSelector) { }
	/**
	 * Resolves the subdomain to an object ID using SuiNS.
	 *
	 * The subdomain `example` will look up `example.sui` and return the object ID if found.
	 */
	async resolveSuiNsAddress(subdomain: string): Promise<string | null> {
		const reqStartTime = Date.now();

		const nameRecord: NameRecord | null = await this.rpcSelector.getNameRecord(
			`${subdomain}.sui`,
		);
		if (nameRecord) {
			const resolvedSuiNSName = nameRecord.walrusSiteId ?? nameRecord.targetAddress;
			logger.info({
				message: "Resolved SuiNS name",
				subdomain,
				resolvedSuiNSName,
			});

			const resolveSuiNsAddressDuration = Date.now() - reqStartTime;
			instrumentationFacade.recordResolveSuiNsAddressTime(
				resolveSuiNsAddressDuration,
				nameRecord.name,
			);

			return resolvedSuiNSName;
		}
		return null;
	}

	hardcodedSubdomains(subdomain: string): string | null {
		if (subdomain in SITE_NAMES) {
			return SITE_NAMES[subdomain];
		}
		return null;
	}
}
