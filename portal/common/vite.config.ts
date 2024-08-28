// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { defineConfig } from 'vite';

export default defineConfig({
    assetsInclude: ["**/*.html"],
    test: {
        onConsoleLog: () => false,
    }
});
