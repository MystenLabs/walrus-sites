#!/usr/bin/env bash
# Copyright (c) Mysten Labs, Inc.
# SPDX-License-Identifier: Apache-2.0

# SEW-893 scenario: aggregator answers with 503 in 8s, inside the 10s
# per-attempt abort window. The fetch resolves normally and is classified
# retry-same. Guards the idleTimeout sizing — the chain completes before
# Bun would otherwise cut the inbound socket at its 10s default.

set -euo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"
run_scenario "fail503" "fail503"
