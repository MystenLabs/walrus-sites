// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { has } from '@vercel/edge-config';
import BlocklistChecker from "@lib/blocklist_checker";

/**
* Defines a blocklistChecker that is integrated with Vercel's Edge Config.
* This means that the blocklistChecker will check if a domain is in the blocklist
* by calling the Vercel Edge Config API.
*
* @returns {BlocklistChecker} The blocklistChecker that is integrated with Vercel's Edge Config.
*/
function create_blocklist_checker(): BlocklistChecker {
    const blocklistChecker = new BlocklistChecker(
        (id: string) => {
            console.log(`Checking if the "${id}" suins domain is in the blocklist...`);
            return has(id)
        }
    );
    return blocklistChecker;
}

let blocklistChecker: BlocklistChecker | undefined;
if (process.env.NODE_ENV === "production") {
    blocklistChecker = create_blocklist_checker();
}
export default blocklistChecker;
