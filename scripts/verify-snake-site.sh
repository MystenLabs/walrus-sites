#!/bin/bash
# Copyright (c) Mysten Labs, Inc.
# SPDX-License-Identifier: Apache-2.0
#
# Verify a published snake example site is working correctly.
#
# Checks: index loads, custom headers, ignored files are 404, routes work.
#
# Usage: verify-snake-site.sh <base-url>
# Example: verify-snake-site.sh http://localhost:3000

set -euo pipefail

BASE_URL="${1:?Usage: $0 <base-url>}"
# Strip trailing slash
BASE_URL="${BASE_URL%/}"

PASS=0
FAIL=0

log() { printf "\033[1;34m==> %s\033[0m\n" "$*"; }
pass() { printf "  \033[1;32m✓ %s\033[0m\n" "$*"; PASS=$((PASS + 1)); }
fail() { printf "  \033[1;31m✗ %s\033[0m\n" "$*"; FAIL=$((FAIL + 1)); }

assert_status() {
    local path="$1" expected="$2" label="$3"
    local status
    status=$(curl -s -o /dev/null -w "%{http_code}" --max-time 10 "${BASE_URL}${path}" 2>/dev/null) || status="000"
    if [[ "$status" == "$expected" ]]; then
        pass "$label"
    else
        fail "$label (got $status, expected $expected)"
    fi
}

assert_header() {
    local path="$1" header="$2" expected="$3" label="$4"
    local value
    value=$(curl -s -D- -o /dev/null --max-time 10 "${BASE_URL}${path}" 2>/dev/null \
        | grep -i "^${header}:" | sed 's/^[^:]*: //' | tr -d '\r') || value=""
    if [[ "$value" == *"$expected"* ]]; then
        pass "$label"
    else
        fail "$label (got '$value', expected to contain '$expected')"
    fi
}

assert_body_contains() {
    local path="$1" needle="$2" label="$3"
    local body
    body=$(curl -s --max-time 10 "${BASE_URL}${path}" 2>/dev/null) || body=""
    if echo "$body" | grep -q "$needle"; then
        pass "$label"
    else
        fail "$label (body does not contain '$needle')"
    fi
}

log "Verifying snake site at $BASE_URL"
echo ""

# 1. Index page loads
log "Index page"
assert_status "/" 200 "GET / returns 200"
assert_body_contains "/" "the Walrus Game" "Body contains expected title"

# 2. Custom headers on index
log "Custom headers"
assert_header "/index.html" "Cache-Control" "max-age=3500" \
    "index.html has Cache-Control: max-age=3500"

# 3. SVG resource with custom headers
log "SVG resource"
assert_status "/file.svg" 200 "GET /file.svg returns 200"
assert_header "/file.svg" "Cache-Control" "public, max-age=86400" \
    "file.svg has Cache-Control: public, max-age=86400"

# 4. Ignored file not published
log "Ignored resources"
assert_status "/secret.txt" 404 "GET /secret.txt returns 404 (ignored)"
assert_status "/private/data.txt" 404 "GET /private/data.txt returns 404 (ignored dir)"

# 5. Route works
log "Routes"
assert_status "/path/anything" 200 "GET /path/anything returns 200 (route match)"
assert_body_contains "/path/anything" "<svg" "Route /path/* serves SVG content"

# Summary
echo ""
log "Results: $PASS passed, $FAIL failed"

if [[ $FAIL -gt 0 ]]; then
    exit 1
fi
