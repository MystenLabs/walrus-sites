// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import type { NextConfig } from 'next'

const nextConfig: NextConfig = {
   transpilePackages: [
       'src/walrus-sites-sw.ts'
   ]
}

export default nextConfig
