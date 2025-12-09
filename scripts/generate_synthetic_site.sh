# Copyright (c) Mysten Labs, Inc.
# SPDX-License-Identifier: Apache-2.0

#!/bin/bash

# Script to generate synthetic websites for performance testing
# Usage: ./generate_synthetic_site.sh <output_dir> <num_files> <size_per_file_kb>

set -e

OUTPUT_DIR="${1:-./synthetic-site}"
NUM_FILES="${2:-10}"
SIZE_PER_FILE_KB="${3:-10}"

# Create output directory if it does not already exist
mkdir -p "$OUTPUT_DIR"

# Validate inputs
if ! [[ "$NUM_FILES" =~ ^[0-9]+$ ]] || [ "$NUM_FILES" -lt 1 ]; then
    echo "Error: Number of files must be a positive integer"
    exit 1
fi

if ! [[ "$SIZE_PER_FILE_KB" =~ ^[0-9]+$ ]] || [ "$SIZE_PER_FILE_KB" -lt 1 ]; then
    echo "Error: Size per file must be a positive integer (in KB)"
    exit 1
fi

SIZE_PER_FILE_BYTES=$(($SIZE_PER_FILE_KB * 1024))
fallocate() { head -c $SIZE_PER_FILE_BYTES /dev/zero > "$1"; }

echo "Generating synthetic website:"
echo "  Output directory: $OUTPUT_DIR"
echo "  Number of files: $NUM_FILES"
echo "  Size per file: ${SIZE_PER_FILE_KB}KB"
echo "  Total size: $((NUM_FILES * SIZE_PER_FILE_KB))KB"

for i in $(seq 0 $((NUM_FILES - 1))); do
    fallocate "$OUTPUT_DIR/file_$i.txt"
done

echo ""
echo "Generated $NUM_FILES files in '$OUTPUT_DIR' totaling $((NUM_FILES * SIZE_PER_FILE_KB))KB."
echo "You can now use this generated content for performance testing."
