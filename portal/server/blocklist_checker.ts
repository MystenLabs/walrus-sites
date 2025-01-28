// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { has } from '@vercel/edge-config';
import BlocklistChecker from "@lib/blocklist_checker";
import { config } from 'configuration_loader';

/**
* Defines a blocklistChecker that is integrated with Vercel's Edge Config.
* This means that the blocklistChecker will check if a domain is in the blocklist
* by calling the Vercel Edge Config API.
*
* @returns {BlocklistChecker} The blocklistChecker that is integrated with Vercel's Edge Config.
*/
function create_vercel_blocklist_checker(): BlocklistChecker {
    const blocklistChecker = new BlocklistChecker(
        (id: string) => {
            console.log(`Checking if the "${id}" suins domain is in the blocklist...`);
            return has(id)
        }
    );
    return blocklistChecker;
}

/**
* Creates a blocklistChecker that uses a Redis database to check domains.
* This implementation will check if a domain is in the blocklist by connecting
* to a Redis server and querying the 'blocklist' set.
*
* @returns {BlocklistChecker} The blocklistChecker that is integrated with Redis.
*/
function create_redis_blocklist_checker(): BlocklistChecker {
    // Implement this function to create a blocklist checker that uses a Redis database.
    throw new Error("Not implemented");
}

let blocklistChecker: BlocklistChecker | undefined;
if (config.enableBlocklist) {
    blocklistChecker = create_vercel_blocklist_checker();
}
export default blocklistChecker;
