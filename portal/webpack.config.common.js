// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

const path = require("path");
const CopyPlugin = require("copy-webpack-plugin");

module.exports = {
    watch: true,
    entry: {
        "walrus-sites-sw": "./src/walrus-sites-sw.ts",
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
        extensions: [".ts", ".js"],
    },
    plugins: [
        new CopyPlugin({
            patterns: [
                {
                    from: "static",
                    globOptions: { ignore: ["**/*.template.html"] },
                },
            ],
        }),
    ],
};
