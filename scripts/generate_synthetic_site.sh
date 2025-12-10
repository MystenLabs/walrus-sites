# Copyright (c) Mysten Labs, Inc.
# SPDX-License-Identifier: Apache-2.0

#!/bin/bash

# Script to generate synthetic websites for performance testing
# Usage: ./generate_synthetic_site.sh --output-dir <dir> --files <size1_b> <size2_b> ...
# Example: Generate a directory named `./synthetic-site` containing 6 files, each sized 100B, 150B, etc.
# |-> ./generate_synthetic_site.sh --output-dir ./synthetic-site --files 100 150 110 120 130 150

set -e

OUTPUT_DIR="./synthetic-site"
FILE_SIZES=()

# Parse arguments
while [[ $# -gt 0 ]]; do
    case "$1" in
        --output-dir)
            OUTPUT_DIR="$2"
            shift 2
            ;;
        --files)
            shift
            while [[ $# -gt 0 && ! "$1" =~ ^-- ]]; do
                FILE_SIZES+=("$1")
                shift
            done
            ;;
        *)
            echo "Unknown option: $1"
            echo "Usage: $0 --output-dir <dir> --files <size1_b> <size2_b> ..."
            exit 1
            ;;
    esac
done

# Validate inputs
if [ ${#FILE_SIZES[@]} -eq 0 ]; then
    echo "Error: At least one file size must be specified"
    echo "Usage: $0 --output-dir <dir> --files <size1_b> <size2_b> ..."
    exit 1
fi

TOTAL_SIZE_B=0
for size in "${FILE_SIZES[@]}"; do
    if ! [[ "$size" =~ ^[0-9]+$ ]] || [ "$size" -lt 1 ]; then
        echo "Error: File size must be a positive integer (in B): $size"
        exit 1
    fi
    TOTAL_SIZE_B=$((TOTAL_SIZE_B + size))
done

# Create output directory if it does not already exist
mkdir -p "$OUTPUT_DIR"

fallocate_file() {
    head -c $1 /dev/urandom > "$2"
}

NUM_FILES=${#FILE_SIZES[@]}

echo "Generating synthetic website:"
echo "  Output directory: $OUTPUT_DIR"
echo "  Number of files: $NUM_FILES"
echo "  File sizes (B): ${FILE_SIZES[*]}"
echo "  Total size: ${TOTAL_SIZE_B}B"

for i in "${!FILE_SIZES[@]}"; do
    fallocate_file "${FILE_SIZES[$i]}" "$OUTPUT_DIR/file_$i.txt"
done

echo ""
echo "Generated $NUM_FILES files in '$OUTPUT_DIR' totaling ${TOTAL_SIZE_B}B."
echo "You can now use this generated content for performance testing."
