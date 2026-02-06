// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import blocklistChecker from "src/blocklist_checker";

export default async function blocklist_healthcheck() {
    // if blocklist is disabled, return true and skip the ping
    // otherwise ping the checker
    const blocklistCheckerPing = blocklistChecker ? await blocklistChecker.ping() : true;
    return new Response(
        JSON.stringify({
            status: blocklistCheckerPing ? "ok" : "error",
        }),
        {
            headers: {
                "Content-Type": "application/json",
            },
        },
    );
}
