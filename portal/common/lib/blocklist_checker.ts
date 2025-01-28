// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

/**
* Determines if a given object or suins domain is in the blocklist.
*/
interface BlocklistChecker {
    /**
    * Checks if the object id of a walrus site object is in the blocklist.
    * @param id: The object id or suins domain to check if it is in the blocklist.
    * @returns True if the id or suins domain is in the blocklist, false otherwise.
    */
    check: (id: string) => Promise<boolean>
    close?: () => void;
}

export default BlocklistChecker;
