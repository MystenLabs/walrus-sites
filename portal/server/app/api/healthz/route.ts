// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import blocklist_healthcheck from "src/blocklist_healthcheck";

export async function GET() {
    return blocklist_healthcheck();
}
