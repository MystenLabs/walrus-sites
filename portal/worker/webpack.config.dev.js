// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

const { merge } = require("webpack-merge");
const common = require("./webpack.config.common.js");

module.exports = merge(common, {
    mode: "development",
});
