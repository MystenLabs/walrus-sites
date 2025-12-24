# Copyright (c) Mysten Labs, Inc.
# SPDX-License-Identifier: Apache-2.0

#!/bin/bash

# Simulates a page edit + page addition in an MkDocs build output
#
# What happens in a real MkDocs page edit + add:
# 1. The edited page's HTML file changes
# 2. A new HTML page is created
# 3. search_index.json is updated with new content
# 4. sitemap.xml and sitemap.xml.gz are updated
#
# This script simulates this by:
# 1. Modifying an existing HTML file's content
# 2. Creating a new HTML page
# 3. Updating search_index.json
# 4. Updating sitemap.xml
#
# Usage: ./modify_mkdocs.sh --site-dir <build-dir>

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

TIMESTAMP=$(date +%s)
NEW_PAGE_NAME="perf-test-page-$TIMESTAMP"

# Helper function for portable sed -i
sed_inplace() {
    if [[ "$OSTYPE" == "darwin"* ]]; then
        sed -i '' "$@"
    else
        sed -i "$@"
    fi
}

echo "=== Simulating MkDocs page edit + add ==="

# 1. Find and modify an existing HTML file (index.html or any page)
HTML_FILE=$(find "$SITE_DIR" -name "index.html" -type f | head -1)
if [ -z "$HTML_FILE" ]; then
    echo "Error: No HTML files found in $SITE_DIR" >&2
    exit 1
fi

echo "Modifying: $HTML_FILE"
# Add a comment at the end of the HTML file
echo "<!-- Modified at $TIMESTAMP for perf test -->" >> "$HTML_FILE"

# 2. Create a new HTML page
NEW_PAGE_DIR="$SITE_DIR/$NEW_PAGE_NAME"
mkdir -p "$NEW_PAGE_DIR"
NEW_PAGE_PATH="$NEW_PAGE_DIR/index.html"

echo "Creating: $NEW_PAGE_PATH"
cat > "$NEW_PAGE_PATH" << EOF
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Performance Test Page - $TIMESTAMP</title>
</head>
<body>
    <h1>Performance Test Page</h1>
    <p>This page was created at $TIMESTAMP for update testing.</p>
    <p>Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.</p>
</body>
</html>
EOF

# 3. Update search_index.json if it exists
SEARCH_INDEX="$SITE_DIR/search/search_index.json"
if [ -f "$SEARCH_INDEX" ]; then
    echo "Updating: $SEARCH_INDEX"
    # Add a new entry to the search index (simple append before closing bracket)
    # First, remove the trailing ] and add new entry
    sed_inplace 's/\]$//' "$SEARCH_INDEX"
    cat >> "$SEARCH_INDEX" << EOF
,{"location":"$NEW_PAGE_NAME/","text":"Performance Test Page. This page was created at $TIMESTAMP for update testing.","title":"Performance Test Page"}]
EOF
fi

# 4. Update sitemap.xml if it exists
SITEMAP="$SITE_DIR/sitemap.xml"
if [ -f "$SITEMAP" ]; then
    echo "Updating: $SITEMAP"
    # Add new URL entry before </urlset>
    sed_inplace "s|</urlset>|<url><loc>https://example.com/$NEW_PAGE_NAME/</loc></url></urlset>|" "$SITEMAP"
fi

# 5. Regenerate sitemap.xml.gz if it exists
SITEMAP_GZ="$SITE_DIR/sitemap.xml.gz"
if [ -f "$SITEMAP" ] && [ -f "$SITEMAP_GZ" ]; then
    echo "Regenerating: $SITEMAP_GZ"
    gzip -c "$SITEMAP" > "$SITEMAP_GZ"
fi

echo "=== MkDocs modification complete ==="
echo "Files affected:"
echo "  - 1 HTML file modified"
echo "  - 1 HTML file created ($NEW_PAGE_NAME/index.html)"
if [ -f "$SEARCH_INDEX" ]; then
    echo "  - search_index.json updated"
fi
if [ -f "$SITEMAP" ]; then
    echo "  - sitemap.xml updated"
fi
if [ -f "$SITEMAP_GZ" ]; then
    echo "  - sitemap.xml.gz regenerated"
fi
