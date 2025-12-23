# Copyright (c) Mysten Labs, Inc.
# SPDX-License-Identifier: Apache-2.0

#!/bin/bash

# Simulates a code edit in a Vite/React build output
#
# What happens in a real Vite code edit:
# 1. Multiple JS chunk files change (new content hashes = new filenames)
# 2. index.html is updated with new JS/CSS references
# 3. Old JS files are removed
#
# This script simulates this by:
# 1. Finding JS files and renaming them with new hashes
# 2. Updating index.html to reference the new filenames
# 3. Modifying the content of the renamed JS files
# 4. Optionally adding an image file (for code-edit-image scenario)
#
# Usage: ./modify_vite.sh --site-dir <build-dir> [--add-image]

set -e

SITE_DIR=""
ADD_IMAGE=false

while [[ $# -gt 0 ]]; do
    case "$1" in
        --site-dir)
            SITE_DIR="$2"
            shift 2
            ;;
        --add-image)
            ADD_IMAGE=true
            shift
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

ASSETS_DIR="$SITE_DIR/assets"
if [ ! -d "$ASSETS_DIR" ]; then
    echo "Error: assets directory not found in $SITE_DIR" >&2
    exit 1
fi

# Generate a new hash suffix (8 characters)
NEW_HASH=$(head -c 100 /dev/urandom | md5 | head -c 8)
TIMESTAMP=$(date +%s)

echo "=== Simulating Vite code edit ==="
echo "New hash suffix: $NEW_HASH"

# Track files for HTML update
declare -a OLD_NAMES
declare -a NEW_NAMES

# 1. Find main entry JS file (usually index-HASH.js, referenced in HTML)
INDEX_HTML="$SITE_DIR/index.html"
if [ ! -f "$INDEX_HTML" ]; then
    echo "Error: index.html not found in $SITE_DIR" >&2
    exit 1
fi

# Extract the main JS file from index.html
MAIN_JS=$(grep -o 'src="/assets/index-[^"]*\.js"' "$INDEX_HTML" | sed 's/src="\/assets\///;s/"$//' | head -1)

if [ -n "$MAIN_JS" ]; then
    MAIN_JS_PATH="$ASSETS_DIR/$MAIN_JS"
    if [ -f "$MAIN_JS_PATH" ]; then
        # Extract base name (index) and create new name
        NEW_MAIN_JS="index-${NEW_HASH}.js"
        NEW_MAIN_JS_PATH="$ASSETS_DIR/$NEW_MAIN_JS"

        echo "Renaming: $MAIN_JS -> $NEW_MAIN_JS"
        mv "$MAIN_JS_PATH" "$NEW_MAIN_JS_PATH"
        echo "// Modified at $TIMESTAMP for perf test" >> "$NEW_MAIN_JS_PATH"

        OLD_NAMES+=("$MAIN_JS")
        NEW_NAMES+=("$NEW_MAIN_JS")
    fi
fi

# 2. Find and rename other JS chunks (lazy-loaded modules)
CHUNK_COUNT=0
for js_file in "$ASSETS_DIR"/*.js; do
    [ -f "$js_file" ] || continue
    basename=$(basename "$js_file")

    # Skip if already processed
    if [[ " ${OLD_NAMES[*]} " =~ " ${basename} " ]]; then
        continue
    fi

    # Only process files that look like chunks (contain a hash)
    if [[ "$basename" =~ ^[a-zA-Z]+-[A-Za-z0-9]+\.js$ ]]; then
        # Extract the chunk name prefix
        CHUNK_PREFIX=$(echo "$basename" | sed 's/-[^-]*\.js$//')
        NEW_CHUNK="${CHUNK_PREFIX}-${NEW_HASH}.js"
        NEW_CHUNK_PATH="$ASSETS_DIR/$NEW_CHUNK"

        echo "Renaming: $basename -> $NEW_CHUNK"
        mv "$js_file" "$NEW_CHUNK_PATH"
        echo "// Modified at $TIMESTAMP for perf test" >> "$NEW_CHUNK_PATH"

        OLD_NAMES+=("$basename")
        NEW_NAMES+=("$NEW_CHUNK")
        CHUNK_COUNT=$((CHUNK_COUNT + 1))

        # Limit to a few chunks to keep it manageable
        [ $CHUNK_COUNT -ge 3 ] && break
    fi
done

# 3. Update index.html with new filenames
echo "Updating index.html with new references..."
for i in "${!OLD_NAMES[@]}"; do
    sed -i '' "s|${OLD_NAMES[$i]}|${NEW_NAMES[$i]}|g" "$INDEX_HTML"
done

# 4. If this is the code-edit-image scenario, add an image
if [ "$ADD_IMAGE" = true ]; then
    IMG_DIR="$ASSETS_DIR"
    IMG_FILE="$IMG_DIR/perf-test-image-$TIMESTAMP.png"

    echo "Adding image: $IMG_FILE"
    # Create a minimal valid PNG (1x1 red pixel)
    printf '\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR\x00\x00\x00\x01\x00\x00\x00\x01\x08\x02\x00\x00\x00\x90wS\xde\x00\x00\x00\x0cIDATx\x9cc\xf8\x0f\x00\x00\x01\x01\x00\x05\x18\xd8N\x00\x00\x00\x00IEND\xaeB`\x82' > "$IMG_FILE"
fi

echo "=== Vite modification complete ==="
echo "Files affected:"
echo "  - ${#NEW_NAMES[@]} JS files renamed and modified"
echo "  - index.html updated"
[ "$ADD_IMAGE" = true ] && echo "  - 1 image file added"
