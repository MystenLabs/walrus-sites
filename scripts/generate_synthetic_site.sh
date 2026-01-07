#!/bin/bash
# Copyright (c) Mysten Labs, Inc.
# SPDX-License-Identifier: Apache-2.0

# Script to generate synthetic websites for performance testing
#
# Usage: ./generate_synthetic_site.sh --output-dir <dir> --file <name:size> [--file <name:size> ...]
#
# Example: Generate a synthetic Sui dApp site
#   ./generate_synthetic_site.sh --output-dir ./synthetic-site \
#     --file vendor.js:320000 \
#     --file app.js:80000 \
#     --file styles.css:30000 \
#     --file assets/logo.png:50000

set -e

OUTPUT_DIR="./synthetic-site"
FILES=()

# Parse arguments
while [[ $# -gt 0 ]]; do
    case "$1" in
        --output-dir)
            OUTPUT_DIR="$2"
            shift 2
            ;;
        --file)
            FILES+=("$2")
            shift 2
            ;;
        *)
            echo "Unknown option: $1"
            echo "Usage: $0 --output-dir <dir> --file <name:size> [--file <name:size> ...]"
            exit 1
            ;;
    esac
done

# Validate inputs
if [ ${#FILES[@]} -eq 0 ]; then
    echo "Error: At least one file must be specified"
    echo "Usage: $0 --output-dir <dir> --file <name:size> [--file <name:size> ...]"
    exit 1
fi

generate_file() {
    local size=$1
    local filepath=$2

    # Create parent directory if needed
    local dir=$(dirname "$filepath")
    mkdir -p "$dir"

    head -c "$size" /dev/urandom > "$filepath"
}

# Create output directory
mkdir -p "$OUTPUT_DIR"

TOTAL_SIZE_B=0
NUM_FILES=0

echo "Generating synthetic website:"
echo "  Output directory: $OUTPUT_DIR"

for entry in "${FILES[@]}"; do
    # Parse name:size format
    if [[ "$entry" =~ ^([^:]+):([0-9]+)$ ]]; then
        name="${BASH_REMATCH[1]}"
        size="${BASH_REMATCH[2]}"

        if [ "$size" -lt 1 ]; then
            echo "Error: File size must be a positive integer: $entry"
            exit 1
        fi

        filepath="$OUTPUT_DIR/$name"
        echo "  - $name (${size}B)"
        generate_file "$size" "$filepath"

        TOTAL_SIZE_B=$((TOTAL_SIZE_B + size))
        NUM_FILES=$((NUM_FILES + 1))
    else
        echo "Error: Invalid file format '$entry'. Expected 'name:size' (e.g., 'app.js:80000')"
        exit 1
    fi
done

echo ""
echo "Generated $NUM_FILES files in '$OUTPUT_DIR' totaling ${TOTAL_SIZE_B}B."
