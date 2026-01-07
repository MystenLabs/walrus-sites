#!/bin/bash
# Copyright (c) Mysten Labs, Inc.
# SPDX-License-Identifier: Apache-2.0

# Script to get SUI and WAL balances for an address
#
# Usage: ./get_balance.sh [--address <address>] [--json] [--output-prefix <prefix>]
#
# Options:
#   --address <address>  Sui address to check (default: active address from sui client)
#   --json               Output in JSON format
#   --output-prefix      Prefix for GitHub Actions output variables (e.g., "initial" -> "initial_sui")
#
# Examples:
#   ./get_balance.sh                           # Get balance for active address
#   ./get_balance.sh --address 0x123...        # Get balance for specific address
#   ./get_balance.sh --json                    # Output as JSON
#   ./get_balance.sh --output-prefix initial   # Set GitHub Actions outputs with prefix

set -e

# Coin type addresses
SUI_COIN="0x2::sui::SUI"
WAL_COIN="0x8270feb7375eee355e64fdb69c50abb6b5f9393a722883c1cf45f8e26048810a::wal::WAL"

ADDRESS=""
JSON_OUTPUT=false
OUTPUT_PREFIX=""

# Parse arguments
while [[ $# -gt 0 ]]; do
    case "$1" in
        --address)
            ADDRESS="$2"
            shift 2
            ;;
        --json)
            JSON_OUTPUT=true
            shift
            ;;
        --output-prefix)
            OUTPUT_PREFIX="$2"
            shift 2
            ;;
        *)
            echo "Unknown option: $1" >&2
            echo "Usage: $0 [--address <address>] [--json] [--output-prefix <prefix>]" >&2
            exit 1
            ;;
    esac
done

# Get balance JSON from sui client
if [ -n "$ADDRESS" ]; then
    BALANCE_JSON=$(sui client balance --address "$ADDRESS" --json)
else
    BALANCE_JSON=$(sui client balance --json)
    ADDRESS=$(sui client active-address)
fi

# Parse balances using jq
# The balance JSON structure: [[address, [[coin_type, [{coinType, balance, ...}]]]]]
SUI_BALANCE=$(echo "$BALANCE_JSON" | jq -r --arg coin "$SUI_COIN" \
    '[.[0][][1][] | select(.coinType == $coin) | .balance | tonumber] | add // 0')
WAL_BALANCE=$(echo "$BALANCE_JSON" | jq -r --arg coin "$WAL_COIN" \
    '[.[0][][1][] | select(.coinType == $coin) | .balance | tonumber] | add // 0')

# Convert to human-readable format (divide by 10^9)
SUI_HUMAN=$(echo "scale=9; $SUI_BALANCE / 1000000000" | bc | sed 's/^\./0./')
WAL_HUMAN=$(echo "scale=9; $WAL_BALANCE / 1000000000" | bc | sed 's/^\./0./')

# Output results
if [ "$JSON_OUTPUT" = true ]; then
    cat <<EOF
{
  "address": "$ADDRESS",
  "sui": {
    "balance": $SUI_BALANCE,
    "human": "$SUI_HUMAN",
    "unit": "MIST"
  },
  "wal": {
    "balance": $WAL_BALANCE,
    "human": "$WAL_HUMAN",
    "unit": "units"
  }
}
EOF
else
    echo "Address: $ADDRESS"
    echo "SUI: $SUI_HUMAN ($SUI_BALANCE MIST)"
    echo "WAL: $WAL_HUMAN ($WAL_BALANCE units)"
fi

# Write to GitHub Actions output if prefix is specified and GITHUB_OUTPUT is set
if [ -n "$OUTPUT_PREFIX" ] && [ -n "$GITHUB_OUTPUT" ]; then
    echo "${OUTPUT_PREFIX}_sui=$SUI_BALANCE" >> "$GITHUB_OUTPUT"
    echo "${OUTPUT_PREFIX}_wal=$WAL_BALANCE" >> "$GITHUB_OUTPUT"
    echo "${OUTPUT_PREFIX}_sui_human=$SUI_HUMAN" >> "$GITHUB_OUTPUT"
    echo "${OUTPUT_PREFIX}_wal_human=$WAL_HUMAN" >> "$GITHUB_OUTPUT"
fi
