#!/usr/bin/env bash
# Copyright (c) Mysten Labs, Inc.
# SPDX-License-Identifier: Apache-2.0

# SEW-893 scenario: aggregator sleeps 30s — longer than the 10s per-attempt
# abort window. AbortSignal.timeout fires, fetch throws, the catch returns
# retry-next, and the executor walks past the slow URL. Guards the
# `signal: AbortSignal.timeout(...)` wire-up itself — deleting it would let
# the chain hang past idleTimeout and the upstream proxy would emit its
# own body in place of the portal's aggregatorFail() response.

set -euo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"
run_scenario "timeout" "sleep" "30000"
