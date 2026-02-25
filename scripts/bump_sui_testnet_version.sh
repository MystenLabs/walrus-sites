#!/bin/bash
# Copyright (c) Mysten Labs, Inc.
# SPDX-License-Identifier: Apache-2.0

# Bump all Sui testnet version references in the repository.
#
# This script modifies files in-place and regenerates lock files.
# It does NOT perform any git or GitHub operations — those belong in the CI workflow.
#
# Usage: ./scripts/bump_sui_testnet_version.sh <new-tag>
# Example: ./scripts/bump_sui_testnet_version.sh testnet-v1.66.0

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"

usage() {
    echo "Usage: $0 <new-tag>"
    echo "Example: $0 testnet-v1.66.0"
    exit 1
}

if [[ $# -ne 1 ]]; then
    usage
fi

NEW_TAG="$1"

# Validate tag format.
if [[ ! "$NEW_TAG" =~ ^testnet-v[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    echo "Error: tag must match format 'testnet-vX.Y.Z', got '$NEW_TAG'"
    exit 1
fi

echo "Bumping Sui testnet version to $NEW_TAG"

# 1. Update tag references in site-builder/Cargo.toml.
echo "Updating site-builder/Cargo.toml ..."
sed -i -E "s|(tag = \")testnet-v[0-9]+\.[0-9]+\.[0-9]+|\1${NEW_TAG}|g" \
    "$REPO_ROOT/site-builder/Cargo.toml"

# 2. Update SUI_TAG in .github/workflows/code.yml.
echo "Updating .github/workflows/code.yml ..."
sed -i -E "s|(SUI_TAG: )testnet-v[0-9]+\.[0-9]+\.[0-9]+|\1${NEW_TAG}|g" \
    "$REPO_ROOT/.github/workflows/code.yml"

# 3. Update VERSION= in .github/workflows/move-tests.yml.
echo "Updating .github/workflows/move-tests.yml ..."
sed -i -E "s|(VERSION=)testnet-v[0-9]+\.[0-9]+\.[0-9]+|\1${NEW_TAG}|g" \
    "$REPO_ROOT/.github/workflows/move-tests.yml"

# 4. Regenerate Cargo.lock.
echo "Regenerating Cargo.lock ..."
(cd "$REPO_ROOT" && cargo check)

# 5. Regenerate Move.lock files.
# Find all Move.toml files and run `sui move build` in their directories.
while IFS= read -r move_toml; do
    move_dir="$(dirname "$move_toml")"
    echo "Regenerating Move.lock in $move_dir ..."
    (cd "$move_dir" && sui move build)
done < <(find "$REPO_ROOT/move" -name "Move.toml" -type f)

echo "Done. All Sui testnet version references updated to $NEW_TAG."
