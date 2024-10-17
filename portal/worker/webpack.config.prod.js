// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

const { merge } = require("webpack-merge");
const common = require("./webpack.config.common.js");
const HtmlMinimizerPlugin = require("html-minimizer-webpack-plugin");
const CssMinimizerPlugin = require("css-minimizer-webpack-plugin");
const TerserPlugin = require("terser-webpack-plugin");

module.exports = merge(common, {
    mode: "production",
    optimization: {
        minimizer: [`...`, new HtmlMinimizerPlugin(), new CssMinimizerPlugin()],
    },
    plugins: [
        new TerserPlugin({
            terserOptions: {
                compress: {
                    // Remove console logs
                    drop_console: ["log"],
                },
            },
        }),
    ],
});
