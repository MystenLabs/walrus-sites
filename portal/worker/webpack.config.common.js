// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

const path = require("path");
const CopyPlugin = require("copy-webpack-plugin");

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
