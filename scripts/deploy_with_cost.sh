# Copyright (c) Mysten Labs, Inc.
# SPDX-License-Identifier: Apache-2.0

#!/bin/bash

# Script to deploy a site and track SUI/WAL costs
#
# Usage: ./deploy_with_cost.sh --site-name <name> --site-dir <dir> [--epochs <n>] [--gas-budget <n>]
#
# Options:
#   --site-name <name>   Name of the site (for output/reporting)
#   --site-dir <dir>     Directory containing the site to deploy
#   --epochs <n>         Number of epochs (default: 1)
#   --gas-budget <n>     Gas budget in MIST (default: 1000000000)
#   --context <ctx>      Sui context (default: testnet)
#
# Outputs (to GITHUB_OUTPUT if set):
#   <site_name>_sui_cost      - SUI cost in MIST
#   <site_name>_wal_cost      - WAL cost in units
#   <site_name>_sui_cost_human - SUI cost in human readable format
#   <site_name>_wal_cost_human - WAL cost in human readable format
#
# Example:
#   ./deploy_with_cost.sh --site-name walrus-docs --site-dir ./walrus-docs-site

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

SITE_NAME=""
SITE_DIR=""
EPOCHS=1
GAS_BUDGET=1000000000
CONTEXT="testnet"

# Parse arguments
while [[ $# -gt 0 ]]; do
    case "$1" in
        --site-name)
            SITE_NAME="$2"
            shift 2
            ;;
        --site-dir)
            SITE_DIR="$2"
            shift 2
            ;;
        --epochs)
            EPOCHS="$2"
            shift 2
            ;;
        --gas-budget)
            GAS_BUDGET="$2"
            shift 2
            ;;
        --context)
            CONTEXT="$2"
            shift 2
            ;;
        *)
            echo "Unknown option: $1" >&2
            exit 1
            ;;
    esac
done

# Validate required arguments
if [ -z "$SITE_NAME" ] || [ -z "$SITE_DIR" ]; then
    echo "Error: --site-name and --site-dir are required" >&2
    echo "Usage: $0 --site-name <name> --site-dir <dir>" >&2
    exit 1
fi

if [ ! -d "$SITE_DIR" ]; then
    echo "Error: Site directory does not exist: $SITE_DIR" >&2
    exit 1
fi

# Get balance before deploy
echo "Getting balance before $SITE_NAME deploy..."
BEFORE_JSON=$("$SCRIPT_DIR/get_balance.sh" --json)
BEFORE_SUI=$(echo "$BEFORE_JSON" | jq -r '.sui.balance')
BEFORE_WAL=$(echo "$BEFORE_JSON" | jq -r '.wal.balance')

# Deploy site
echo "Deploying $SITE_NAME..."
./target/release/site-builder --context "$CONTEXT" --gas-budget "$GAS_BUDGET" deploy "$SITE_DIR" --epochs "$EPOCHS"

# Get balance after deploy
echo "Getting balance after $SITE_NAME deploy..."
AFTER_JSON=$("$SCRIPT_DIR/get_balance.sh" --json)
AFTER_SUI=$(echo "$AFTER_JSON" | jq -r '.sui.balance')
AFTER_WAL=$(echo "$AFTER_JSON" | jq -r '.wal.balance')

# Calculate costs
SUI_COST=$((BEFORE_SUI - AFTER_SUI))
WAL_COST=$((BEFORE_WAL - AFTER_WAL))
SUI_COST_HUMAN=$(echo "scale=9; $SUI_COST / 1000000000" | bc | sed 's/^\./0./')
WAL_COST_HUMAN=$(echo "scale=9; $WAL_COST / 1000000000" | bc | sed 's/^\./0./')

# Output results
echo ""
echo "=== $SITE_NAME Deploy Cost ==="
echo "SUI: $SUI_COST_HUMAN ($SUI_COST MIST)"
echo "WAL: $WAL_COST_HUMAN ($WAL_COST units)"

# Write to GitHub Actions output if GITHUB_OUTPUT is set
if [ -n "$GITHUB_OUTPUT" ]; then
    echo "${SITE_NAME}_sui_cost=$SUI_COST" >> "$GITHUB_OUTPUT"
    echo "${SITE_NAME}_wal_cost=$WAL_COST" >> "$GITHUB_OUTPUT"
    echo "${SITE_NAME}_sui_cost_human=$SUI_COST_HUMAN" >> "$GITHUB_OUTPUT"
    echo "${SITE_NAME}_wal_cost_human=$WAL_COST_HUMAN" >> "$GITHUB_OUTPUT"
fi
