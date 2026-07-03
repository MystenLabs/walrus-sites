// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { describe, it, expect } from "vitest";
import { SuiGrpcClient } from "@mysten/sui/grpc";
import { SUI_CLOCK_OBJECT_ID } from "@mysten/sui/utils";
import { isObjectNotFoundError } from "@lib/rpc_selector";

/**
 * Drift guard for the gRPC `core.getObjects` shape, checked against mainnet — the
 * canonical, stable network (testnet is periodically reset).
 *
 * `RPCSelector.multiGetObjects` depends on a specific contract from the SDK +
 * fullnode, and silently relies on it: for a batch of object ids, `getObjects`
 * returns a single `{ objects: [...] }` array, in request order, where
 *   - a MISSING object is an `Error` ELEMENT (not thrown, not omitted), which each
 *     caller classifies: resource.ts turns it into a 404, routing keeps it as
 *     `Routes | Error`, and getNameRecord resolves the thrown variant to `null`, and
 *   - a PRESENT object carries the fields we read (`content` bytes, `version`).
 *
 * This test pins that contract by hitting a real fullnode with one guaranteed
 * object (the Clock, 0x6, exists on every network) and one guaranteed-missing
 * id. If a Sui SDK / fullnode bump changes how a per-object miss is signalled
 * (e.g. it starts throwing, or returns null), this goes red — go update the
 * mapping in `multiGetObjects` (rpc_selector.ts) to match the new shape.
 *
 * Requires MAINNET_RPC_URL (set via .env.test). We error rather than skip when
 * it's absent, so this guard can't silently vanish.
 */
const mainnetRpc = process.env.MAINNET_RPC_URL;
if (!mainnetRpc) {
    throw new Error(
        "MAINNET_RPC_URL must be set to run the gRPC drift guard (normally provided by .env.test)",
    );
}
// Full-width improbable id: low addresses are reserved namespace that framework
// releases keep claiming (0xd became the Display registry), so "never exists"
// erodes there.
const MISSING_OBJECT_ID = "0x" + "deadbeef".repeat(8); // valid-shaped id, never exists

describe("gRPC getObjects shape (mainnet drift guard)", () => {
    it("returns a present object and an Error element for a miss, in order", async () => {
        const client = new SuiGrpcClient({ baseUrl: mainnetRpc, network: "mainnet" });

        const { objects } = await client.core.getObjects({
            objectIds: [SUI_CLOCK_OBJECT_ID, MISSING_OBJECT_ID],
            include: { content: true, display: true },
        });

        // Order is preserved: result[i] corresponds to objectIds[i].
        expect(objects).toHaveLength(2);

        // The present object is NOT an Error and carries the fields we read.
        const present = objects[0];
        expect(
            present instanceof Error,
            `expected Clock (0x6) to be present, got error: ${(present as Error)?.message}`,
        ).toBe(false);
        const obj = present as Exclude<typeof present, Error>;
        expect(obj.content).toBeInstanceOf(Uint8Array);
        expect(typeof obj.version).toBe("string");

        // The missing object surfaces as an Error ELEMENT — the signal every
        // caller keys off (resource 404, routing's `Routes | Error`, getNameRecord's
        // null). If this stops being an Error, those paths would leak a malformed
        // object instead.
        expect(
            objects[1] instanceof Error,
            "SDK no longer returns a per-object miss as an Error element — revisit multiGetObjects mapping",
        ).toBe(true);

        // Pin the miss message shape: callers (e.g. url_fetcher's routes/redirects
        // logging) use isObjectNotFoundError to tell an expected miss from an
        // unexpected error. If this fails, the SDK/fullnode changed the message —
        // update isObjectNotFoundError in rpc_selector.ts.
        expect(
            isObjectNotFoundError(objects[1]),
            `isObjectNotFoundError no longer detects a per-object miss: ${
                (objects[1] as Error)?.message
            }`,
        ).toBe(true);
    }, 30_000);
});
