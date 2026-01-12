// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { defineConfig, loadEnv } from 'vite';
import path from 'path';
import { readFileSync } from 'fs';

// Custom plugin to load HTML files as raw text strings
// This ensures HTML template imports with { type: "text" } work correctly in tests
function rawHtmlPlugin() {
	return {
		name: 'raw-html-loader',
		transform(_code: string, id: string) {
			if (id.endsWith('.html')) {
				const content = readFileSync(id, 'utf-8');
				return {
					code: `export default ${JSON.stringify(content)};`,
					map: null,
				};
			}
		},
	};
}

export default defineConfig(({ mode }) => ({
	plugins: [rawHtmlPlugin()],
	assetsInclude: ["**/*.html"],
	resolve: {
		alias: {
			'@lib': path.resolve(__dirname, 'lib/src'),
			'@templates': path.resolve(__dirname, 'html_templates'),
		},
	},
	test: {
		include: ['lib/tests/**/*.test.ts'],
		onConsoleLog: () => false,
		env: loadEnv(mode, process.cwd(), ''),
		coverage: {
			provider: 'v8',
			reporter: [
				['lcov', { 'projectRoot': './src' }],
				['json', { 'file': 'coverage.json' }],
				['text']
			]
		},
	},
}));
