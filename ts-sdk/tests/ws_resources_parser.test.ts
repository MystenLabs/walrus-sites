// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { describe, it, expect } from 'bun:test'
import { parseWsResources } from '@utils/ws_resources_parser'

describe('parseWsResources', () => {
    it('should parse valid ws-resources.json', () => {
        const data = {
            headers: {
                '/*.svg': {
                    'Cache-Control': 'public, max-age=86400',
                    ETag: '"abc123"',
                },
                '/index.html': {
                    'Cache-Control': 'max-age=3500',
                    'Content-Type': 'text/html; charset=utf-8',
                },
            },
            routes: {
                '/path/*': '/file.svg',
            },
            metadata: {
                link: 'https://docs.wal.app',
                image_url: 'https://www.walrus.xyz/walrus-site',
                description: 'This is a walrus site.',
                project_url: 'https://github.com/MystenLabs/walrus-sites/',
                creator: 'MystenLabs',
            },
            site_name: 'Walrus Snake Game',
            object_id: '0x4a1be0fb330215c532d74c70d34bc35f185cc7ce025e04b9ad42bc4ac8eda5ce',
            ignore: ['/private', '/private/*', '/secret.txt'],
        }

        const result = parseWsResources(data)

        expect(result).toEqual(data)
    })

    it('should throw on invalid input', () => {
        expect(() => parseWsResources({ headers: 'invalid' })).toThrow()
        expect(() => parseWsResources({ routes: [] })).toThrow()
        expect(() => parseWsResources({ metadata: 123 })).toThrow()
    })
})
