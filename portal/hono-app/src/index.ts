// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { Hono } from 'hono'

const app = new Hono()

app.get('/', (c) => {
    return c.text('Hello Hono!')
})

export default app
