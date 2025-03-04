// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { Hono } from "hono";
import redisClient from "./redis";
import { bearerAuth } from "hono/bearer-auth";
import { except } from 'hono/combine'

enum STATUS {
	OK = 200,
	NOT_FOUND = 404,
	CREATED = 201,
	DELETED = 200,
	INTERNAL_SERVER_ERROR = 500,
}

const token = process.env.BEARER_TOKEN;
if (!token) {
	throw new Error("BEARER_TOKEN environment variable is not set");
}
const app = new Hono();
app.use('/*', except('/health', bearerAuth({ token })))

app.get("/", async (c) => {
	return c.text(
		"Blocklist API ready. Use GET/PUT/DELETE operations with /:domain to manage entries.",
	STATUS.OK);
});

app.get("/health", async (c) => {
    const ping = await redisClient.ping();
    if (!ping) {
        return c.text("UNHEALTHY", STATUS.INTERNAL_SERVER_ERROR);
    }
	return c.text("OK", STATUS.OK);
});

app.get("/:domain", async (c) => {
	const { domain } = c.req.param();
	const exists = await redisClient.exists(domain);
	if (exists) {
		console.info(`(GET) found: ${domain}`);
		return c.text(`Domain found: ${domain}`, STATUS.OK);
	}
	console.info(`(GET) not found: ${domain}`);
	return c.text(`Domain not found: ${domain}`, STATUS.NOT_FOUND);
});

app.put("/:domain", async (c) => {
	const { domain } = c.req.param();
	console.info(`(PUT) domain added: ${domain}`);
	await redisClient.set(domain);
	return c.text(`Received domain: ${domain}`, STATUS.CREATED);
});

app.delete("/:domain", async (c) => {
	const { domain } = c.req.param();
	await redisClient.delete(domain);
	console.info(`(DELETE) removed: ${domain}`);
	return c.text(`Deleted domain: ${domain}`, STATUS.OK);
});

export default app;
