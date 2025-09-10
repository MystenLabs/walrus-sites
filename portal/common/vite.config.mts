// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { defineConfig, loadEnv } from 'vite';
import path from 'path';

export default defineConfig(({ mode }) => ({
	assetsInclude: ["**/*.html"],
	resolve: {
		alias: {
			'@lib': path.resolve(__dirname, 'lib/src'),
			'@templates': path.resolve(__dirname, 'html_templates'),
		},
	},
	test: {
		onConsoleLog: () => false,
		env: loadEnv(mode, process.cwd(), ''),
		coverage: {
			provider: 'v8', // or 'v8'
			reporter: [
				['lcov', { 'projectRoot': './src' }],
				['json', { 'file': 'coverage.json' }],
				['text']
			]
		},
	},
}));
