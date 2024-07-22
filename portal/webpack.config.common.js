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
        alias: {
          "@helpers": path.resolve(__dirname, "src/helpers"),
          "@static": path.resolve(__dirname, "static"),
        },
        extensions: [".ts", ".js"],
    },
    plugins: [
        new CopyPlugin({
            patterns: [
                {
                    from: "static",
                    globOptions: { ignore: ["**/*.template.html"] },
                },
                {
                    // Duplicate index.html in the destination folder as 404.html
                    from: "static/index.html",
                    to: "404.html",
                }
            ],
        }),
    ],
};
