// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import type { SuiCodegenConfig } from "@mysten/codegen";

const config: SuiCodegenConfig = {
    output: "./contracts/sites",
    generateSummaries: true,
    prune: true,
    packages: [
        {
            package: "@walrus/sites",
            path: "../move/walrus_site",
        },
    ],
};

export default config;
