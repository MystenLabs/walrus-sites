// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0
import { SITE_NAMES } from "@lib/constants";
import { RPCSelector } from "@lib/rpc_selector";
import logger from "@lib/logger";
import { NameRecord } from "@lib/types";
import { instrumentationFacade } from "@lib/instrumentation";

export class SuiNSResolver {
    constructor(private rpcSelector: RPCSelector) {}
    /**
    * Resolves the subdomain to an object ID using SuiNS.
    *
    * The subdomain `example` will look up `example.sui` and return the object ID if found.
    */
    async resolveSuiNsAddress(subdomain: string
    ): Promise<string | null> {
    	logger.info("Resolving SuiNS domain", {suinsDomain: subdomain})
    	const reqStartTime = Date.now();
        const nameRecord: NameRecord | null = await this.rpcSelector.getNameRecord(`${subdomain}.sui`);
        if (nameRecord) {
            const resolvedSuiNSObjectId = nameRecord.walrusSiteId;
            logger.info("Resolved SuiNS name", {subdomain, resolvedSuiNSObjectId});
            const resolveSuiNsAddressDuration = Date.now() - reqStartTime;
			instrumentationFacade.recordResolveSuiNsAddressTime(
				resolveSuiNsAddressDuration,
				nameRecord.name,
			);
            return resolvedSuiNSObjectId;
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
