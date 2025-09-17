// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { describe, it, expect } from "vitest";
import { sha256 } from "@lib/crypto";
import { toBase64 } from "@mysten/bcs";

describe("sha256", () => {
	it("hashing the a string should always yield the same result", async () => {
		const arrayBuffer = new TextEncoder().encode("Decentralise the web!").buffer as ArrayBuffer;
		const res = await sha256(arrayBuffer)
		const resString = toBase64(res);
		expect(resString).toEqual("yml5ecL3vnssrx78HsvpBypPVrsyhtk0XPGrVOYTNT4=");
	});
});
