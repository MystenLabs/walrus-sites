// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

const path = require("path");
const webpack = require("webpack");
const CopyPlugin = require("copy-webpack-plugin");
require('dotenv').config({ path: '.env.local' });

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
            "@lib": path.resolve(__dirname, "../common/lib"),
            "@static": path.resolve(__dirname, "../common/static"),
        },
        extensions: [".ts", ".js", ".html"],
    },
    plugins: [
        new webpack.DefinePlugin({
            'process.env.PORTAL_DOMAIN_NAME_LENGTH': JSON.stringify(
                process.env.PORTAL_DOMAIN_NAME_LENGTH || undefined
            ),
            'process.env.RPC_URL_LIST': JSON.stringify(
                process.env.RPC_URL_LIST || undefined
            )
        }),
        new CopyPlugin({
            patterns: [
                {
                    from: "../common/static",
                    globOptions: { ignore: ["**/*.template.html"] },
                },
                {
                    // Duplicate index.html in the destination folder as 404.html
                    from: "../common/static/index.html",
                    to: "404.html",
                }
            ],
        }),
    ],
};
