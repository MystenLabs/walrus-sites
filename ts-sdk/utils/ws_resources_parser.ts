// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { z } from 'zod'

const MetadataSchema = z.object({
    link: z.string().optional(),
    image_url: z.string().optional(),
    description: z.string().optional(),
    project_url: z.string().optional(),
    creator: z.string().optional(),
})

const WsResourcesConfigSchema = z.object({
    headers: z.record(z.string(), z.record(z.string(), z.string())).optional(),
    routes: z.record(z.string(), z.string()).optional(),
    metadata: MetadataSchema.optional(),
    site_name: z.string().optional(),
    object_id: z.string().optional(),
    ignore: z.array(z.string()).optional(),
})

export type WsResourcesConfig = z.infer<typeof WsResourcesConfigSchema>

export function parseWsResources(data: unknown): WsResourcesConfig {
    return WsResourcesConfigSchema.parse(data)
}
