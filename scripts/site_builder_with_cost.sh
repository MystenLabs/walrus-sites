#!/bin/bash
# Copyright (c) Mysten Labs, Inc.
# SPDX-License-Identifier: Apache-2.0

# Generic wrapper to run site-builder commands with cost/time tracking
#
# NOTE: This script uses GNU time (/usr/bin/time -v) for memory tracking,
#       which is only available on Linux (e.g., GitHub Actions Ubuntu runners).
#       It will not work on macOS without installing GNU time (gtime).
#
# Usage: ./site_builder_with_cost.sh --output-prefix <prefix> -- <site-builder args...>
#
# Options:
#   --output-prefix <p>  Prefix for output variables (required)
#   --context <ctx>      Sui context (default: testnet)
#   --                   Separator - everything after this is passed to site-builder
#
# Outputs (to GITHUB_OUTPUT if set):
#   <prefix>_time           - Duration in seconds
#   <prefix>_sui_cost       - SUI cost in MIST
#   <prefix>_wal_cost       - WAL cost in units
#   <prefix>_sui_cost_human - SUI cost in human readable format
#   <prefix>_wal_cost_human - WAL cost in human readable format
#   <prefix>_peak_memory_mb - Peak memory usage in MB
#   <prefix>_user_cpu_time  - User CPU time in seconds (excludes I/O wait)
#   <prefix>_site_id        - Site object ID (if ws-resources.json exists)
#
# Examples:
#   # Deploy
#   ./site_builder_with_cost.sh --output-prefix walrus_docs_deploy -- \
#       deploy ./walrus-docs-site --epochs 1
#
#   # Update
#   ./site_builder_with_cost.sh --output-prefix walrus_docs_update1 -- \
#       update ./walrus-docs-site --epochs 1

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

OUTPUT_PREFIX=""
CONTEXT="testnet"
GAS_BUDGET="1000000000"
SITE_BUILDER_ARGS=()

# Parse arguments until --
while [[ $# -gt 0 ]]; do
    case "$1" in
        --output-prefix)
            OUTPUT_PREFIX="$2"
            shift 2
            ;;
        --context)
            CONTEXT="$2"
            shift 2
            ;;
        --gas-budget)
            GAS_BUDGET="$2"
            shift 2
            ;;
        --)
            shift
            SITE_BUILDER_ARGS=("$@")
            break
            ;;
        *)
            echo "Unknown option: $1" >&2
            exit 1
            ;;
    esac
done

# Validate
if [ -z "$OUTPUT_PREFIX" ]; then
    echo "Error: --output-prefix is required" >&2
    exit 1
fi

if [ ${#SITE_BUILDER_ARGS[@]} -eq 0 ]; then
    echo "Error: No site-builder arguments provided after --" >&2
    exit 1
fi

# Extract site directory from args (second arg for deploy/update commands)
COMMAND="${SITE_BUILDER_ARGS[0]}"
SITE_DIR="${SITE_BUILDER_ARGS[1]}"

echo "=== Running: site-builder $COMMAND ==="
echo "Output prefix: $OUTPUT_PREFIX"

# Get balance before
echo "Getting balance before operation..."
BEFORE_JSON=$("$SCRIPT_DIR/get_balance.sh" --json)
BEFORE_SUI=$(echo "$BEFORE_JSON" | jq -r '.sui.balance')
BEFORE_WAL=$(echo "$BEFORE_JSON" | jq -r '.wal.balance')

# Run site-builder with GNU time for metrics (time, memory, CPU)
TIME_OUTPUT_FILE=$(mktemp)
trap "rm -f $TIME_OUTPUT_FILE" EXIT

/usr/bin/time -v ./target/release/site-builder --context "$CONTEXT" --gas-budget "$GAS_BUDGET" "${SITE_BUILDER_ARGS[@]}" 2> >(tee "$TIME_OUTPUT_FILE" >&2)

# Extract all metrics from GNU time output
OP_TIME=$(grep "Elapsed (wall clock) time" "$TIME_OUTPUT_FILE" | awk '{print $NF}')
PEAK_MEMORY_KB=$(grep "Maximum resident set size" "$TIME_OUTPUT_FILE" | awk '{print $NF}')
PEAK_MEMORY_MB=$((PEAK_MEMORY_KB / 1024))
USER_CPU_TIME=$(grep "User time (seconds)" "$TIME_OUTPUT_FILE" | awk '{print $NF}')

echo "Duration: ${OP_TIME}"
echo "Peak memory: ${PEAK_MEMORY_MB} MB"
echo "User CPU time: ${USER_CPU_TIME}s"

# Get balance after
echo "Getting balance after operation..."
AFTER_JSON=$("$SCRIPT_DIR/get_balance.sh" --json)
AFTER_SUI=$(echo "$AFTER_JSON" | jq -r '.sui.balance')
AFTER_WAL=$(echo "$AFTER_JSON" | jq -r '.wal.balance')

# Calculate costs
SUI_COST=$((BEFORE_SUI - AFTER_SUI))
WAL_COST=$((BEFORE_WAL - AFTER_WAL))
SUI_COST_HUMAN=$(echo "scale=9; $SUI_COST / 1000000000" | bc | sed 's/^\./0./')
WAL_COST_HUMAN=$(echo "scale=9; $WAL_COST / 1000000000" | bc | sed 's/^\./0./')

# Try to extract site ID if ws-resources.json exists
SITE_ID=""
if [ -n "$SITE_DIR" ] && [ -f "$SITE_DIR/ws-resources.json" ]; then
    SITE_ID=$(jq -r '.object_id // empty' "$SITE_DIR/ws-resources.json")
fi

# Output results
echo ""
echo "=== Results ==="
echo "Time: ${OP_TIME}"
echo "User CPU time: ${USER_CPU_TIME}s"
echo "Peak memory: ${PEAK_MEMORY_MB} MB"
echo "SUI: $SUI_COST_HUMAN ($SUI_COST MIST)"
echo "WAL: $WAL_COST_HUMAN ($WAL_COST units)"
[ -n "$SITE_ID" ] && echo "Site ID: $SITE_ID"

# Write to GitHub Actions output if GITHUB_OUTPUT is set
if [ -n "$GITHUB_OUTPUT" ]; then
    echo "${OUTPUT_PREFIX}_time=$OP_TIME" >> "$GITHUB_OUTPUT"
    echo "${OUTPUT_PREFIX}_user_cpu_time=$USER_CPU_TIME" >> "$GITHUB_OUTPUT"
    echo "${OUTPUT_PREFIX}_peak_memory_mb=$PEAK_MEMORY_MB" >> "$GITHUB_OUTPUT"
    echo "${OUTPUT_PREFIX}_sui_cost=$SUI_COST" >> "$GITHUB_OUTPUT"
    echo "${OUTPUT_PREFIX}_wal_cost=$WAL_COST" >> "$GITHUB_OUTPUT"
    echo "${OUTPUT_PREFIX}_sui_cost_human=$SUI_COST_HUMAN" >> "$GITHUB_OUTPUT"
    echo "${OUTPUT_PREFIX}_wal_cost_human=$WAL_COST_HUMAN" >> "$GITHUB_OUTPUT"
    [ -n "$SITE_ID" ] && echo "${OUTPUT_PREFIX}_site_id=$SITE_ID" >> "$GITHUB_OUTPUT"
fi
