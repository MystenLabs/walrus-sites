#!/usr/bin/env bash
# Copyright (c) Mysten Labs, Inc.
# SPDX-License-Identifier: Apache-2.0

# Shared helpers for the SEW-893 e2e regression scenarios. Sourced — not
# executed directly. Defines the chain (mock-aggregator + portal + envoy)
# start/wait/assert flow; per-scenario scripts call `run_scenario` with
# the MODE (and optionally SLEEP_MS) they want the mock to use.
#
# Each scenario asserts that the portal returns its own 503
# "Failed to contact the aggregator" body — *not* Envoy's
# `connection_termination` body that surfaced before the fix.

set -euo pipefail

LIB_HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PORTAL_DIR="$LIB_HERE/../../portal"
LOG_ROOT="$LIB_HERE/.logs"

TARGET="http://46f3881sp4r55fc6pcao9t93bieeejl4vr4k2uv8u4wwyx1a93.localhost:4000/index.html"
EXPECTED_STATUS=503
EXPECTED_BODY_SUBSTR="Failed to contact the aggregator"
# Cap how long we'll wait for the portal to finish exhausting the retry chain.
# With default config (2 URLs, 3+1 attempts, 8s mock delay, 10s abort timeout,
# 500ms inter-retry delay), the worst case is ~33s — round to 60s.
CURL_MAX_TIME=60

mkdir -p "$LOG_ROOT"

cleanup_processes() {
    lsof -ti :3000 -ti :4000 -ti :8080 2>/dev/null | xargs -r kill -9 2>/dev/null || true
}

wait_for() {
    local name="$1" probe="$2"
    for _ in $(seq 1 60); do
        if eval "$probe" >/dev/null 2>&1; then return 0; fi
        sleep 0.5
    done
    echo "::error::$name did not come up within 30s"
    return 1
}

# Args:
#   $1  label     — used for the per-scenario log dir name
#   $2  mode      — MODE env var passed to mock-aggregator.ts
#   $3  sleep_ms  — optional SLEEP_MS env var (only meaningful in MODE=sleep)
run_scenario() {
    local label="$1" mode="$2" sleep_ms="${3:-}"
    local scenario_log="$LOG_ROOT/$label"
    mkdir -p "$scenario_log"
    rm -f "$scenario_log"/*.log "$scenario_log"/response.body

    trap cleanup_processes EXIT
    cleanup_processes

    echo
    echo "=== scenario: $label (MODE=$mode${sleep_ms:+ SLEEP_MS=$sleep_ms}) ==="

    command -v envoy >/dev/null || { echo "envoy not in PATH"; return 1; }
    [[ -d "$PORTAL_DIR/node_modules" ]] \
        || (cd "$PORTAL_DIR" && bun install --frozen-lockfile)

    echo "starting mock-aggregator..."
    MODE="$mode" SLEEP_MS="${sleep_ms:-}" \
        bun run "$LIB_HERE/mock-aggregator.ts" >"$scenario_log/mock.log" 2>&1 &

    echo "starting portal..."
    (cd "$PORTAL_DIR" && PORTAL_CONFIG="$LIB_HERE/portal-config.yaml" bun -F server start) \
        >"$scenario_log/portal.log" 2>&1 &

    echo "starting envoy..."
    envoy -c "$LIB_HERE/envoy.yaml" >"$scenario_log/envoy.log" 2>&1 &

    # Probe each upstream before firing the test request — otherwise a failed
    # mock/envoy start surfaces as a buried connection error later. The mock
    # deliberately stalls HTTP responses (per MODE), so for it and envoy we
    # only verify the listening socket via TCP connect.
    wait_for "mock-aggregator" "bash -c '</dev/tcp/localhost/8080'" \
        || { echo "see $scenario_log/mock.log"; return 1; }
    wait_for "portal" "curl -sf -o /dev/null --max-time 1 http://localhost:3000/__wal__/healthz" \
        || { echo "see $scenario_log/portal.log"; return 1; }
    wait_for "envoy" "bash -c '</dev/tcp/localhost/4000'" \
        || { echo "see $scenario_log/envoy.log"; return 1; }

    echo "firing request at $TARGET"
    local response_file="$scenario_log/response.body"
    local http_status
    http_status=$(curl -sS \
        -o "$response_file" \
        -w "%{http_code}" \
        --max-time "$CURL_MAX_TIME" \
        "$TARGET" || echo "curl_failed")

    echo "portal responded with HTTP $http_status"
    echo "body:"
    sed 's/^/  /' "$response_file"

    if [[ "$http_status" != "$EXPECTED_STATUS" ]]; then
        echo "::error::[$label] expected HTTP $EXPECTED_STATUS, got $http_status"
        return 1
    fi
    if ! grep -q "$EXPECTED_BODY_SUBSTR" "$response_file"; then
        echo "::error::[$label] expected body to contain '$EXPECTED_BODY_SUBSTR'"
        return 1
    fi
    echo "OK [$label]: portal returned its own aggregatorFail() response"
}
