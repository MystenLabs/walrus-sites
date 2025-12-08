// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { describe, it, expect } from 'bun:test'
import { parseSitesConfig } from '@utils/sites_config_parser'

describe('parseSitesConfig', () => {
    it('should parse valid sites-config', () => {
        const data = {
            contexts: {
                testnet: {
                    package: '0xf99aee9f21493e1590e7e5a9aea6f343a1f381031a04a732724871fc294be799',
                    staking_object:
                        '0xbe46180321c30aab2f8b3501e24048377287fa708018a5b7c2792b35fe339ee3',
                    general: {
                        wallet_env: 'testnet',
                        walrus_context: 'testnet',
                        walrus_package:
                            '0xd84704c17fc870b8764832c535aa6b11f21a95cd6f5bb38a9b07d2cf42220c66',
                    },
                },
                mainnet: {
                    package: '0x26eb7ee8688da02c5f671679524e379f0b837a12f1d1d799f255b7eea260ad27',
                    staking_object:
                        '0x10b9d30c28448939ce6c4d6c6e0ffce4a7f8a4ada8248bdad09ef8b70e4a3904',
                    general: {
                        wallet_env: 'mainnet',
                        walrus_context: 'mainnet',
                        walrus_package:
                            '0xfdc88f7d7cf30afab2f82e8380d11ee8f70efb90e863d1de8616fae1bb09ea77',
                    },
                },
            },
            default_context: 'mainnet',
        }

        const result = parseSitesConfig(data)

        expect(result.default_context).toBe('mainnet')
        expect(result.contexts.testnet!.package).toBe(
            '0xf99aee9f21493e1590e7e5a9aea6f343a1f381031a04a732724871fc294be799'
        )
    })

    it('should throw on invalid input', () => {
        expect(() => parseSitesConfig({ contexts: 'invalid' })).toThrow()
        expect(() => parseSitesConfig({ contexts: {}, default_context: 123 })).toThrow()
        expect(() =>
            parseSitesConfig({ contexts: { testnet: { package: 'missing_required_fields' } } })
        ).toThrow()
    })
})
