// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { describe, expect, test } from 'vitest'
import { getDomain, getSubdomainAndPath } from './domain_parsing'
import { Path } from '../types'

const getDomainTestCases: [string, string][] = [
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
    getDomainTestCases.forEach(([input, expected]) => {
        test(`${input} -> ${expected}`, () => {
            const domain = getDomain(input)
                expect(domain).toEqual(expected)
        })
    })
})

const getSubdomainAndPathTestCases: [string, Path][] = [
    ['https://subname.name.walrus.site/', {subdomain: 'subname.name', path: '/index.html' }],
    ['https://name.walrus.site/', { subdomain: 'name', path: '/index.html' }],
    ['https://name.localhost:8080/', { subdomain: 'name', path: '/index.html' }],
    ['https://subsubname.subname.suinsname.portalname.co.uk/',
      { subdomain: 'subsubname.subname.suinsname', path: '/index.html' }],
]

describe('getSubdomainAndPath', () => {
  getSubdomainAndPathTestCases.forEach(
    ([input, path]) => {
    test(`${input} ->
      subdomain: ${path.subdomain ?? "null"},
      path: ${path.path ?? "null"}`,
      () => {
        expect(getSubdomainAndPath(new URL(input))).toEqual(path);
    });
  });
})
