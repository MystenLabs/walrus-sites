// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

/**
* Determines if a given object or blob id is in the blocklist.
*/
class BlocklistChecker {
    /// Includes the predicate that determines if a site is in the blocklist.
    private checkBlocklistPredicate: (site: string) => Promise<boolean>;

    /**
    * Constructs a new BlocklistChecker object.
    * @param checkBlocklistPredicate: Defines the predicate to check if
    * a site is in the blocklist.
    */
    constructor(checkBlocklistPredicate: (site: string) => Promise<boolean>) {
        this.checkBlocklistPredicate = checkBlocklistPredicate;
    }

    /**
    * Checks if the object id of a walrus *site* object is in the blocklist.
    * @param id: The object or blob id to check if it is in the blocklist.
    * @returns True if the id is in the blocklist, false otherwise.
    */
    async isBlocked(id: string): Promise<boolean> {
        return await this.checkBlocklistPredicate(id);
    }
}

export default BlocklistChecker;
