// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

/** @type {import('next').NextConfig} */
const nextConfig = {
    webpack: (config, options) => {
        config.module.rules.push({
            test: /\.html$/,
            use: [
                {
                    loader: 'html-loader',
                },
            ],
        })
        return config
    },
}

export default nextConfig
