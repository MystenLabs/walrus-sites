// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

// Slow Walrus aggregator mock for reproducing SEW-893.
//
// MODE=fail503 (default): sleeps FAIL_DELAY_MS, then returns 503. Mirrors
//   production where aggregator-cache takes ~8-11s before failing.
// MODE=headers_then_hang: sends headers, hangs body forever (used for the
//   Envoy connection_termination repro path).
// MODE=sleep: sleeps SLEEP_MS before sending headers.
//
// Env vars:
//   PORT           (default 8080)
//   FAIL_DELAY_MS  (default 8000, used in MODE=fail503)
//   SLEEP_MS       (default 30000, used in MODE=sleep)
//   MODE           fail503 | headers_then_hang | sleep  (default fail503)

declare const Bun: {
    env: Record<string, string | undefined>;
    serve: (opts: {
        port: number;
        idleTimeout?: number;
        fetch: (req: Request) => Promise<Response> | Response;
    }) => unknown;
};

const PORT = Number(Bun.env.PORT ?? 8080);
const SLEEP_MS = Number(Bun.env.SLEEP_MS ?? 30_000);
const FAIL_DELAY_MS = Number(Bun.env.FAIL_DELAY_MS ?? 8_000);
const MODE = Bun.env.MODE ?? "fail503";

const sleep = (ms: number, signal: AbortSignal) =>
    new Promise<void>((resolve, reject) => {
        const t = setTimeout(resolve, ms);
        signal.addEventListener("abort", () => {
            clearTimeout(t);
            reject(new Error("upstream aborted by mock-aggregator client"));
        });
    });

Bun.serve({
    port: PORT,
    idleTimeout: 255,
    async fetch(req: Request) {
        const { pathname } = new URL(req.url);
        const start = Date.now();

        if (MODE === "fail503") {
            console.log(`[mock-aggregator] ${req.method} ${pathname} (will 503 in ${FAIL_DELAY_MS}ms)`);
            try {
                await sleep(FAIL_DELAY_MS, req.signal);
            } catch {
                console.log(`[mock-aggregator] ${pathname} — client gave up after ${Date.now() - start}ms`);
                return new Response("aborted", { status: 499 });
            }
            console.log(`[mock-aggregator] ${pathname} → 503 (after ${Date.now() - start}ms)`);
            return new Response("simulated upstream failure", { status: 503 });
        }

        if (MODE === "sleep") {
            console.log(`[mock-aggregator] ${req.method} ${pathname} — sleep ${SLEEP_MS}ms`);
            try {
                await sleep(SLEEP_MS, req.signal);
                return new Response("dummy", { status: 200 });
            } catch {
                console.log(
                    `[mock-aggregator] ${pathname} — client gave up after ${Date.now() - start}ms`,
                );
                return new Response("aborted", { status: 499 });
            }
        }

        // headers_then_hang: respond with headers + Content-Length immediately,
        // then never write any body chunks. The portal's `await fetch()` resolves
        // fast, but `await response.arrayBuffer()` hangs reading from the stream.
        console.log(`[mock-aggregator] ${req.method} ${pathname} — headers_then_hang`);
        const stream = new ReadableStream({
            start() {},
            cancel() {
                console.log(
                    `[mock-aggregator] ${pathname} — body stream cancelled after ${Date.now() - start}ms`,
                );
            },
        });
        return new Response(stream, {
            status: 200,
            headers: {
                "content-type": "application/octet-stream",
                "content-length": "1048576",
            },
        });
    },
});

console.log(`mock-aggregator listening on :${PORT} (MODE=${MODE}, SLEEP_MS=${SLEEP_MS})`);
