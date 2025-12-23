# Copyright (c) Mysten Labs, Inc.
# SPDX-License-Identifier: Apache-2.0

#!/bin/bash

# Simulates a page edit in a Docusaurus build output
#
# What happens in a real Docusaurus page edit:
# 1. The page's content chunk JS file changes (new content hash = new filename)
# 2. runtime~main.js changes (new content hash = new filename)
# 3. All HTML files update to reference the new runtime~main.js
#
# This script simulates this by:
# 1. Finding a page chunk JS file and renaming it with new hash
# 2. Finding runtime~main.js and renaming it with new hash
# 3. Updating all HTML files to reference the new filenames
# 4. Modifying the content of the renamed JS files
#
# Usage: ./modify_docusaurus.sh --site-dir <build-dir>

set -e

SITE_DIR=""

while [[ $# -gt 0 ]]; do
    case "$1" in
        --site-dir)
            SITE_DIR="$2"
            shift 2
            ;;
        *)
            echo "Unknown option: $1" >&2
            exit 1
            ;;
    esac
done

if [ -z "$SITE_DIR" ]; then
    echo "Error: --site-dir is required" >&2
    exit 1
fi

if [ ! -d "$SITE_DIR" ]; then
    echo "Error: Site directory does not exist: $SITE_DIR" >&2
    exit 1
fi

JS_DIR="$SITE_DIR/assets/js"
if [ ! -d "$JS_DIR" ]; then
    echo "Error: assets/js directory not found in $SITE_DIR" >&2
    exit 1
fi

# Generate a new hash suffix (8 characters)
NEW_HASH=$(head -c 100 /dev/urandom | md5 | head -c 8)
TIMESTAMP=$(date +%s)

echo "=== Simulating Docusaurus page edit ==="
echo "New hash suffix: $NEW_HASH"

# 1. Find runtime~main.HASH.js and rename it
RUNTIME_OLD=$(ls "$JS_DIR"/runtime~main.*.js 2>/dev/null | head -1)
if [ -z "$RUNTIME_OLD" ]; then
    echo "Error: runtime~main.*.js not found" >&2
    exit 1
fi
RUNTIME_OLD_BASENAME=$(basename "$RUNTIME_OLD")
RUNTIME_NEW="runtime~main.${NEW_HASH}.js"
RUNTIME_NEW_PATH="$JS_DIR/$RUNTIME_NEW"

echo "Renaming: $RUNTIME_OLD_BASENAME -> $RUNTIME_NEW"
mv "$RUNTIME_OLD" "$RUNTIME_NEW_PATH"

# Modify runtime content to ensure it's actually changed
echo "// Modified at $TIMESTAMP for perf test" >> "$RUNTIME_NEW_PATH"

# 2. Find a page chunk (any chunk that's not runtime/main) and rename it
PAGE_CHUNK_OLD=$(ls "$JS_DIR"/*.js 2>/dev/null | grep -v "runtime~main" | grep -v "^main" | head -1)
if [ -n "$PAGE_CHUNK_OLD" ]; then
    PAGE_CHUNK_OLD_BASENAME=$(basename "$PAGE_CHUNK_OLD")
    # Extract the chunk ID (first part before the hash)
    CHUNK_ID=$(echo "$PAGE_CHUNK_OLD_BASENAME" | sed 's/\.[^.]*\.js$//')
    PAGE_CHUNK_NEW="${CHUNK_ID}.${NEW_HASH}.js"
    PAGE_CHUNK_NEW_PATH="$JS_DIR/$PAGE_CHUNK_NEW"

    echo "Renaming: $PAGE_CHUNK_OLD_BASENAME -> $PAGE_CHUNK_NEW"
    mv "$PAGE_CHUNK_OLD" "$PAGE_CHUNK_NEW_PATH"

    # Modify chunk content
    echo "// Modified at $TIMESTAMP for perf test" >> "$PAGE_CHUNK_NEW_PATH"
fi

# 3. Update all HTML files to reference the new runtime filename
echo "Updating HTML files with new runtime reference..."
HTML_COUNT=0
for html_file in $(find "$SITE_DIR" -name "*.html" -type f); do
    if grep -q "$RUNTIME_OLD_BASENAME" "$html_file" 2>/dev/null; then
        sed -i '' "s|$RUNTIME_OLD_BASENAME|$RUNTIME_NEW|g" "$html_file"
        HTML_COUNT=$((HTML_COUNT + 1))
    fi
done
echo "Updated $HTML_COUNT HTML files"

# 4. If we renamed a page chunk, update any HTML that references it
if [ -n "$PAGE_CHUNK_OLD" ]; then
    for html_file in $(find "$SITE_DIR" -name "*.html" -type f); do
        if grep -q "$PAGE_CHUNK_OLD_BASENAME" "$html_file" 2>/dev/null; then
            sed -i '' "s|$PAGE_CHUNK_OLD_BASENAME|$PAGE_CHUNK_NEW|g" "$html_file"
        fi
    done
fi

echo "=== Docusaurus modification complete ==="
echo "Files affected:"
echo "  - 1 runtime JS file renamed and modified"
[ -n "$PAGE_CHUNK_OLD" ] && echo "  - 1 page chunk JS file renamed and modified"
echo "  - $HTML_COUNT HTML files updated"
