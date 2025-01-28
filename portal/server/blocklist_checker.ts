// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { has } from '@vercel/edge-config';
import BlocklistChecker from "@lib/blocklist_checker";
import { config } from 'configuration_loader';
import assert from 'assert';

class VercelBlocklistChecker implements BlocklistChecker {
    constructor() {
        assert(
            config.enableBlocklist,
            "Blocklist checker should not be created if blocklist is disabled."
        );
    }

    async check(id: string): Promise<boolean> {
        return has(id);
    }
}

let blocklistChecker: BlocklistChecker | undefined;
if (config.enableBlocklist) {
    blocklistChecker = new VercelBlocklistChecker();
}
export default blocklistChecker;
