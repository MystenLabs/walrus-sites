// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { z } from 'zod'

const GeneralConfigSchema = z.object({
    wallet_env: z.string(),
    walrus_context: z.string(),
    walrus_package: z.string(),
    wallet_address: z.string().optional(),
    rpc_url: z.string().optional(),
    wallet: z.string().optional(),
    walrus_binary: z.string().optional(),
    walrus_config: z.string().optional(),
    gas_budget: z.number().optional(),
})

const ContextConfigSchema = z.object({
    module: z.string().optional(),
    portal: z.string().optional(),
    package: z.string(),
    staking_object: z.string(),
    general: GeneralConfigSchema,
})

const SitesConfigSchema = z.object({
    contexts: z.record(z.string(), ContextConfigSchema),
    default_context: z.enum(['mainnet', 'testnet']),
})

export type GeneralConfig = z.infer<typeof GeneralConfigSchema>
export type ContextConfig = z.infer<typeof ContextConfigSchema>
export type SitesConfig = z.infer<typeof SitesConfigSchema>

export function parseSitesConfig(data: unknown): SitesConfig {
    return SitesConfigSchema.parse(data)
}
