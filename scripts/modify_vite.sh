#!/bin/bash
# Copyright (c) Mysten Labs, Inc.
# SPDX-License-Identifier: Apache-2.0

# Simulates a code edit in a Vite/React build output
#
# What happens in a real Vite code edit:
# 1. Multiple JS chunk files change (new content hashes = new filenames)
# 2. CSS file changes (new content hash = new filename)
# 3. index.html is updated with new JS/CSS references
# 4. Old JS/CSS files are removed
#
# This script simulates this by:
# 1. Modifying the content of JS/CSS files (inserting a unique marker)
# 2. Computing a new hash from the modified content
# 3. Renaming files based on their new content hash
# 4. Updating index.html to reference the new filenames
# 5. Optionally adding an image file (for code-edit-image scenario)
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

TIMESTAMP=$(date +%s)

# Helper function to compute hash from file content (first 8 chars)
compute_content_hash() {
    local file="$1"
    if command -v md5sum &> /dev/null; then
        md5sum "$file" | head -c 8
    else
        md5 -q "$file" | head -c 8
    fi
}

# Helper function for portable sed -i
sed_inplace() {
    if [[ "$OSTYPE" == "darwin"* ]]; then
        sed -i '' "$@"
    else
        sed -i "$@"
    fi
}

echo "=== Simulating Vite code edit ==="

# Track files for HTML update (using pipe-separated strings for portability)
OLD_NAMES=""
NEW_NAMES=""

# Helper to add to tracking lists
add_rename() {
    local old="$1"
    local new="$2"
    if [ -z "$OLD_NAMES" ]; then
        OLD_NAMES="$old"
        NEW_NAMES="$new"
    else
        OLD_NAMES="$OLD_NAMES|$old"
        NEW_NAMES="$NEW_NAMES|$new"
    fi
}

# Helper to modify a file and rename based on content hash
modify_and_rename() {
    local file="$1"
    local prefix="$2"
    local ext="$3"
    local comment_start="$4"
    local comment_end="$5"

    local old_name
    old_name=$(basename "$file")

    # Modify the content by inserting a unique marker at the beginning
    # This simulates real code changes that affect the entire bundle
    local temp_file="${file}.tmp"
    echo "${comment_start} Build marker: $TIMESTAMP ${comment_end}" > "$temp_file"
    cat "$file" >> "$temp_file"
    mv "$temp_file" "$file"

    # Compute new hash from modified content
    local new_hash
    new_hash=$(compute_content_hash "$file")

    # Create new filename
    local new_name="${prefix}-${new_hash}.${ext}"
    local new_path="$ASSETS_DIR/$new_name"

    echo "Modifying and renaming: $old_name -> $new_name"
    mv "$file" "$new_path"

    add_rename "$old_name" "$new_name"
}

INDEX_HTML="$SITE_DIR/index.html"
if [ ! -f "$INDEX_HTML" ]; then
    echo "Error: index.html not found in $SITE_DIR" >&2
    exit 1
fi

# Collect ALL files to process BEFORE any modifications
# This prevents renamed files from being processed again

# 1. Get main entry JS file (usually index-HASH.js, referenced in HTML)
MAIN_JS=$(grep -o 'src="/assets/index-[^"]*\.js"' "$INDEX_HTML" | sed 's/src="\/assets\///;s/"$//' | head -1)
MAIN_JS_PATH=""
if [ -n "$MAIN_JS" ] && [ -f "$ASSETS_DIR/$MAIN_JS" ]; then
    MAIN_JS_PATH="$ASSETS_DIR/$MAIN_JS"
fi

# 2. Get CSS file (usually index-HASH.css, referenced in HTML)
MAIN_CSS=$(grep -o 'href="/assets/index-[^"]*\.css"' "$INDEX_HTML" | sed 's/href="\/assets\///;s/"$//' | head -1)
MAIN_CSS_PATH=""
if [ -n "$MAIN_CSS" ] && [ -f "$ASSETS_DIR/$MAIN_CSS" ]; then
    MAIN_CSS_PATH="$ASSETS_DIR/$MAIN_CSS"
fi

# 3. Collect ALL other JS chunks (lazy-loaded modules)
JS_FILES=""
for js_file in "$ASSETS_DIR"/*.js; do
    [ -f "$js_file" ] || continue
    js_basename=$(basename "$js_file")

    # Skip the main JS file (will be processed separately)
    if [ -n "$MAIN_JS" ] && [ "$js_basename" = "$MAIN_JS" ]; then
        continue
    fi

    # Only process files that look like chunks (contain a hash pattern like name-HASH.js)
    if [[ "$js_basename" =~ ^([a-zA-Z0-9_-]+)-[A-Za-z0-9_-]+\.js$ ]]; then
        JS_FILES="$JS_FILES|$js_file"
    fi
done

# Now process all collected files

# Process main JS
if [ -n "$MAIN_JS_PATH" ]; then
    modify_and_rename "$MAIN_JS_PATH" "index" "js" "//" ""
fi

# Process main CSS
if [ -n "$MAIN_CSS_PATH" ]; then
    modify_and_rename "$MAIN_CSS_PATH" "index" "css" "/*" "*/"
fi

# Process other JS files
IFS='|'
for js_file in $JS_FILES; do
    [ -z "$js_file" ] && continue
    js_basename=$(basename "$js_file")

    # Extract the chunk name prefix
    [[ "$js_basename" =~ ^([a-zA-Z0-9_-]+)-[A-Za-z0-9_-]+\.js$ ]]
    CHUNK_PREFIX="${BASH_REMATCH[1]}"

    modify_and_rename "$js_file" "$CHUNK_PREFIX" "js" "//" ""
done
unset IFS

# 4. Update index.html with new filenames
echo "Updating index.html with new references..."
IFS='|'
old_arr=($OLD_NAMES)
new_arr=($NEW_NAMES)
unset IFS

for i in "${!old_arr[@]}"; do
    sed_inplace "s|${old_arr[$i]}|${new_arr[$i]}|g" "$INDEX_HTML"
done

# 5. If this is the code-edit-image scenario, add an image
if [ "$ADD_IMAGE" = true ]; then
    IMG_DIR="$ASSETS_DIR"
    IMG_FILE="$IMG_DIR/perf-test-image-$TIMESTAMP.png"

    echo "Adding image: $IMG_FILE"
    # Create a minimal valid PNG (1x1 red pixel)
    printf '\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR\x00\x00\x00\x01\x00\x00\x00\x01\x08\x02\x00\x00\x00\x90wS\xde\x00\x00\x00\x0cIDATx\x9cc\xf8\x0f\x00\x00\x01\x01\x00\x05\x18\xd8N\x00\x00\x00\x00IEND\xaeB`\x82' > "$IMG_FILE"
fi

# Count files modified
FILE_COUNT=$(echo "$OLD_NAMES" | tr '|' '\n' | grep -c . || echo 0)

echo "=== Vite modification complete ==="
echo "Files affected:"
echo "  - $FILE_COUNT JS/CSS files modified and renamed"
echo "  - index.html updated"
if [ "$ADD_IMAGE" = true ]; then
    echo "  - 1 image file added"
fi
