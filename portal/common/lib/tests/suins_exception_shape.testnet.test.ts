// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { describe, it, expect } from "vitest";
import { RPCSelector } from "@lib/rpc_selector";
import { parsePriorityUrlList } from "@lib/priority_executor";

/**
 * Integration test — confirms what the @mysten/sui SDK actually throws when
 * a SuiNS name is not registered on testnet. The portal relies on this
 * exception shape to distinguish "name not registered" (healthy FN, just no
 * record) from "FN unreachable".
 *
 * Skipped unless RPC_URL_LIST is set (e.g. via .env.test). Hits real testnet
 * so it's a few seconds and may be flaky if all FNs in the priority list are
 * down at once.
 *
 * Real-world observation: the *primary* FN
 * (https://fullnode.testnet.sui.io) returns ObjectError(code: "notExists",
 * "Object 0x... does not exist"), which is the authoritative answer.
 * Secondary FNs (blastapi, suiet) frequently return SuiHTTPStatusError 403
 * or 502 because they're rate-limited or unauthenticated. The failover
 * machinery tries every URL anyway, so the AggregateError ends up containing
 * both kinds of cause. Our error-handling logic must treat the presence of
 * any "notExists" cause as proof that the name doesn't exist (since name
 * registration is determinate on-chain state).
 */
const rpcEnv = process.env.RPC_URL_LIST;

describe.skipIf(!rpcEnv)("SuiNS exception shape on real testnet", () => {
    const rpcSelector = new RPCSelector(parsePriorityUrlList(rpcEnv ?? ""), "testnet");

    it("returns AggregateError whose causes include ObjectError(notExists) for an unregistered name", async () => {
        // A name we are confident is not registered. Random suffix avoids
        // collisions if someone happens to register it in the future.
        const subdomain = `walrus-portal-test-nonexistent-${Date.now()}`;

        let thrown: unknown;
        try {
            await rpcSelector.getNameRecord(`${subdomain}.sui`);
        } catch (e) {
            thrown = e;
        }

        expect(thrown).toBeInstanceOf(AggregateError);

        const aggregate = thrown as AggregateError;
        const causes = aggregate.errors.map((e: Error) => (e as Error & { cause?: unknown }).cause);

        // At least one cause must be the ObjectError(notExists) thrown by
        // the SDK when the FN cleanly answers "object does not exist". This
        // is the marker our fix uses to discriminate name-not-found from
        // real FN failure.
        const notExistsCauses = causes.filter(
            (c): c is Error & { code: string } =>
                c instanceof Error && (c as Error & { code?: string }).code === "notExists",
        );
        expect(notExistsCauses.length).toBeGreaterThan(0);
        expect(notExistsCauses[0].message).toMatch(/Object 0x[0-9a-f]+ does not exist/i);
    }, 30_000);
});
