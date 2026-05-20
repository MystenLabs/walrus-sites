#!/usr/bin/env bash
# Copyright (c) Mysten Labs, Inc.
# SPDX-License-Identifier: Apache-2.0

# E2E regression test for SEW-893.
#
#   curl  →  envoy (:4000)  →  portal (:3000)  →  mock-aggregator (:8080)
#
# The mock returns 503 after MODE=fail503 (default). With the fix in place
# (AbortSignal.timeout + idleTimeout sized to retry budget), the portal must
# exhaust the aggregator retry chain and reply with its own 503
# "Failed to contact the aggregator" body — *not* Envoy's
# `connection_termination` body it used to surface before the fix.
#
# Exit code 0 = portal returned the expected response. Non-zero = regression.

set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PORTAL_DIR="$HERE/../../portal"
LOG="$HERE/.logs"
TARGET="http://46f3881sp4r55fc6pcao9t93bieeejl4vr4k2uv8u4wwyx1a93.localhost:4000/index.html"
EXPECTED_STATUS=503
EXPECTED_BODY_SUBSTR="Failed to contact the aggregator"
# Cap how long we'll wait for the portal to finish exhausting the retry chain.
# With default config (2 URLs, 3+1 attempts, 8s mock delay, 10s abort timeout,
# 500ms inter-retry delay), the worst case is ~33s — round to 60s.
CURL_MAX_TIME=60

mkdir -p "$LOG"
rm -f "$LOG"/*.log "$LOG"/response.body

cleanup() {
    lsof -ti :3000 -ti :4000 -ti :8080 2>/dev/null | xargs -r kill -9 2>/dev/null || true
}
trap cleanup EXIT
cleanup

command -v envoy >/dev/null || { echo "envoy not in PATH"; exit 1; }
[[ -d "$PORTAL_DIR/node_modules" ]] || (cd "$PORTAL_DIR" && bun install --frozen-lockfile)

echo "starting mock-aggregator (MODE=${MODE:-fail503})..."
bun run "$HERE/mock-aggregator.ts" >"$LOG/mock.log" 2>&1 &

echo "starting portal..."
(cd "$PORTAL_DIR" && PORTAL_CONFIG="$HERE/portal-config.yaml" bun -F server start) \
    >"$LOG/portal.log" 2>&1 &

echo "starting envoy..."
envoy -c "$HERE/envoy.yaml" >"$LOG/envoy.log" 2>&1 &

echo "waiting for portal to come up..."
for _ in $(seq 1 60); do
    if curl -sf -o /dev/null --max-time 1 http://localhost:3000/__wal__/healthz; then
        break
    fi
    sleep 0.5
done
if ! curl -sf -o /dev/null --max-time 1 http://localhost:3000/__wal__/healthz; then
    echo "::error::portal did not come up within 30s"
    exit 1
fi

echo "firing request at $TARGET"
RESPONSE_FILE="$LOG/response.body"
HTTP_STATUS=$(curl -sS \
    -o "$RESPONSE_FILE" \
    -w "%{http_code}" \
    --max-time "$CURL_MAX_TIME" \
    "$TARGET" || echo "curl_failed")

echo "portal responded with HTTP $HTTP_STATUS"
echo "body:"
sed 's/^/  /' "$RESPONSE_FILE"

if [[ "$HTTP_STATUS" != "$EXPECTED_STATUS" ]]; then
    echo "::error::expected HTTP $EXPECTED_STATUS, got $HTTP_STATUS"
    exit 1
fi

if ! grep -q "$EXPECTED_BODY_SUBSTR" "$RESPONSE_FILE"; then
    echo "::error::expected body to contain '$EXPECTED_BODY_SUBSTR'"
    exit 1
fi

echo "OK: portal returned its own aggregatorFail() response"
