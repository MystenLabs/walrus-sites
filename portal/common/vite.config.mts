// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { defineConfig, loadEnv } from 'vite';
import codspeedPlugin from "@codspeed/vitest-plugin";

export default defineConfig(({ mode }) => ({
    assetsInclude: ["**/*.html"],
    test: {
        onConsoleLog: () => false,
        env: loadEnv(mode, process.cwd(), ''),
    },
    plugins: [codspeedPlugin()],
}));
