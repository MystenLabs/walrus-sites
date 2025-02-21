// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

export default interface AllowlistChecker {
    /**
     * Initializes the allowlist checker.
     */
    init: () => Promise<void>;

    /**
     * Checks if the object id or suins domain of a walrus site object is in the allowlist.
     * @param id The object id or suins domain to check if it is in the allowlist.
     * @returns True if the id or suins domain is in the allowlist, false otherwise.
     */
    isAllowed: (id: string) => Promise<boolean>

    /**
     * In case of any cleanup needed for the allowlist checker, this method should be called.
     * Examples of cleanup include closing any open connections or releasing resources.
     */
    close?: () => void;

    /**
     * Pings the allowlist checker to check if it is healthy.
     * @returns True if the allowlist checker is healthy, false otherwise.
     */
    ping: () => Promise<boolean>;
}
