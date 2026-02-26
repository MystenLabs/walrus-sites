// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

const fs = require("fs");
const path = require("path");
const webpack = require("webpack");
const CopyPlugin = require("copy-webpack-plugin");
const YAML = require("yaml");
require('dotenv').config({ path: '.env.local' });

// --- Load YAML config at build time (if available) ---

function loadYamlDefaults() {
    const configPath = process.env.PORTAL_CONFIG || "portal-config.yaml";
    if (!fs.existsSync(configPath)) {
        return {};
    }
    console.log(`[webpack] Loading portal config from ${configPath}`);
    const content = fs.readFileSync(configPath, "utf-8");
    const yaml = YAML.parse(content);

    // TODO: URL arrays are converted to pipe-delimited strings here, then parsed back in the worker
    const toPipeString = (urls) =>
        urls.map((u) => `${u.url}|${u.retries}|${u.metric}`).join(",");

    const defaults = {};
    if (yaml.network) defaults.SUINS_CLIENT_NETWORK = yaml.network;
    if (yaml.site_package) defaults.SITE_PACKAGE = yaml.site_package;
    if (yaml.domain_name_length !== undefined) {
        defaults.PORTAL_DOMAIN_NAME_LENGTH = String(yaml.domain_name_length);
    }
    if (yaml.rpc_urls) defaults.RPC_URL_LIST = toPipeString(yaml.rpc_urls);
    if (yaml.aggregator_urls) defaults.AGGREGATOR_URL_LIST = toPipeString(yaml.aggregator_urls);

    return defaults;
}

const yamlDefaults = loadYamlDefaults();

// Helper: env var wins over YAML default
function envOrYaml(envKey) {
    return process.env[envKey] || yamlDefaults[envKey] || undefined;
}

module.exports = {
    watch: true,
    entry: {
        "walrus-sites-sw": "./src/walrus-sites-sw.ts",
        "walrus-sites-portal-register-sw": "./src/walrus-sites-portal-register-sw.ts",
    },
    module: {
        rules: [
            {
                test: /\.html$/,
                type: "asset/source",
            },
            {
                test: /\.ts$/,
                use: "ts-loader",
                exclude: /node_modules/,
            },
        ],
    },
    output: {
        filename: "[name].js",
        path: path.resolve(__dirname, "./dist/"),
        clean: true,
    },
    resolve: {
        alias: {
            "@lib": path.resolve(__dirname, "../common/lib/src"),
            "@templates": path.resolve(__dirname, "../common/html_templates"),
        },
        extensions: [".ts", ".js", ".html"],
        fallback: {
            "http": require.resolve("stream-http"),
            "https": require.resolve("https-browserify"),
            "stream": require.resolve("stream-browserify"),
            "url": require.resolve("url/"),
            "util": require.resolve("util/"),
        }
    },
    plugins: [
        new webpack.DefinePlugin({
            'process.env.PORTAL_DOMAIN_NAME_LENGTH': JSON.stringify(
                envOrYaml('PORTAL_DOMAIN_NAME_LENGTH')
            ),
            'process.env.RPC_URL_LIST': JSON.stringify(
                envOrYaml('RPC_URL_LIST')
            ),
            'process.env.SUINS_CLIENT_NETWORK': JSON.stringify(
                envOrYaml('SUINS_CLIENT_NETWORK')
            ),
            'process.env.AGGREGATOR_URL_LIST': JSON.stringify(
                envOrYaml('AGGREGATOR_URL_LIST')
            ),
            'process.env.SITE_PACKAGE': JSON.stringify(
                envOrYaml('SITE_PACKAGE')
            ),
        }),
        new CopyPlugin({
            patterns: [
                {
                    from: "./static",
                    globOptions: { ignore: ["**/*.template.html"] },
                },
                {
                    // Duplicate index.html in the destination folder as 404.html
                    from: "./static/index.html",
                    to: "404.html",
                }
            ],
        }),
    ],
};
