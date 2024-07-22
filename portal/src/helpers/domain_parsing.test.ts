/// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { describe, it, expect, test } from 'vitest'
import { getDomain } from './domain_parsing'

const testCases: [string, string][] = [
    ['https://example.com', 'example.com'],
    ['https://suinsname.localhost:8080', 'localhost'],
    ['https://subname.suinsname.localhost:8080', 'localhost'],
    ['https://flatland.walrus.site/', 'walrus.site'],
    ['https://4snh0c0o7quicfzokqpsmuchtgitnukme1q680o1s1nfn325sr.walrus.site/',
        'walrus.site'],
    ['https://4snh0c0o7quicfzokqpsmuchtgitnukme1q680o1s1nfn325sr.portalname.co.uk/',
        'portalname.co.uk'],
    ['https://subname.suinsname.portalname.co.uk/', 'portalname.co.uk'],
    ['https://subsubname.subname.suinsname.portalname.co.uk/',
        'portalname.co.uk']
]

describe('getDomain', () => {
    testCases.forEach(([input, expected]) => {
        test(`${input} -> ${expected}`, () => {
            const domain = getDomain(input)
                expect(domain).toEqual(expected)
        })
    })
})
