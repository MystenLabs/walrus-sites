// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { create_site_and_send_to_sender } from '../src/flows'
import { describe, it, expect } from 'bun:test'
import { SiteBuilder } from '../src/site-builder'
import { Transaction } from '@mysten/sui/transactions'
import { join } from 'path'

describe('site publishing', () => {
    it('should publish a test site', async () => {
        // Read the config
        const configPath = join(process.cwd(), '..', 'sites-config.yaml')
        // Initialise the SiteBuilder (orchestrator) class
        const siteBuilder = new SiteBuilder(configPath)
        // Create an empty transaction. It will be used to add consecutive moveCalls to it.
        const tx = new Transaction()
        // FIXME: expecting to be undefined for a happy path is weird.
        const tx2 = create_site_and_send_to_sender(tx, siteBuilder)
        expect(await siteBuilder.run(tx2)).toBeUndefined()
    })
})
