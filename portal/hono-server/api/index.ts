// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { Hono } from 'hono'
import { handle } from 'hono/vercel'

export const config = {
    runtime: 'edge'
}

const app = new Hono().basePath('/api')

app.get('/', (c) => {
    return c.json({ message: 'Hello Hono!' })
})

export default handle(app)
