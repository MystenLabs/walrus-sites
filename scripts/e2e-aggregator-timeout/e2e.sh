#!/usr/bin/env bash
# Copyright (c) Mysten Labs, Inc.
# SPDX-License-Identifier: Apache-2.0

# E2E regression test for SEW-893. Runs every scenario in sequence; exits
# non-zero on the first failure. Add a new scenario by dropping a sibling
# scenario script and listing it here.

set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
"$HERE/fail503.sh"
