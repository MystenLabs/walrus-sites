// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { expect, it, xit, describe } from 'bun:test'
import { SuiClient } from '@mysten/sui/client'
import { walrus } from '@mysten/walrus'
import { walrusSites } from '@client'

describe('walrusSitesClientShouldBeInitialisable', () => {
    it('initialises the walrus sites client happy path', () => {
        const client = new SuiClient({
            url: 'https://fullnode.testnet.sui.io:443'
        })
        client
            .$extend(walrus())
            .$extend(walrusSites())
    })
})
