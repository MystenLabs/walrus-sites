// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

/**************************************************************
 * THIS FILE IS GENERATED AND SHOULD NOT BE MANUALLY MODIFIED *
 **************************************************************/
import { MoveStruct } from '../utils/index.js'
import { bcs } from '@mysten/sui/bcs'
const $moduleName = '@walrus/sites::events'
export const SiteCreatedEvent = new MoveStruct({
    name: `${$moduleName}::SiteCreatedEvent`,
    fields: {
        site_id: bcs.Address,
    },
})
export const SiteBurnedEvent = new MoveStruct({
    name: `${$moduleName}::SiteBurnedEvent`,
    fields: {
        site_id: bcs.Address,
    },
})
