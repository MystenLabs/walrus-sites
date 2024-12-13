// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { createClient, EdgeConfigClient } from '@vercel/edge-config';
import { config } from 'configuration-loader';

let edgeConfigAllowlistClient: EdgeConfigClient | undefined;
if (config.enableAllowlist){
    edgeConfigAllowlistClient = createClient(config.edgeConfigAllowlist);
}
/**
* Check if a given subdomain is allowed to be served by the walrus site.
* @param subdomain The walrus site subdomain to inspect
* @returns true if the subdomain is allowed (has premium), false otherwise
*/
export async function isAllowed(subdomain: string): Promise<boolean> {
    if (!config.enableAllowlist){
        return false
    }

    if (!edgeConfigAllowlistClient){
        throw new Error('Edge config allowlist client not initialized!')
    }

    const allowed: boolean = await edgeConfigAllowlistClient.has(
       subdomain,
    );

    return allowed
}
