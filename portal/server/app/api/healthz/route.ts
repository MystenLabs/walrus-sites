// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { NextResponse } from "next/server";
import allowlistChecker from "src/allowlist_checker";
import blocklistChecker from "src/blocklist_checker";

export async function GET() {
    // if allowlist or blocklist is disabled, return true and skip the ping
    // otherwise ping the checker
    const allowlistCheckerPing = allowlistChecker ? await allowlistChecker.ping() : true;
    const blocklistCheckerPing = blocklistChecker ? await blocklistChecker.ping() : true;
    const isHealthy = allowlistCheckerPing && blocklistCheckerPing;
    return NextResponse.json({
        status: isHealthy ? "ok" : "error",
    });
}
