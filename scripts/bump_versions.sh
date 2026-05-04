#!/bin/bash
# Copyright (c) Mysten Labs, Inc.
# SPDX-License-Identifier: Apache-2.0

# Bump Sui and/or Walrus dependency versions across the repository.
#
# This script modifies files in-place and regenerates lock files.
# It does NOT perform any git or GitHub operations — those belong in the CI workflow.
#
# Usage: ./scripts/bump_versions.sh [--dry-run] [--sui-tag <tag>] [--walrus-ref <ref>]
#
# At least one of --sui-tag or --walrus-ref is required.
#
# Options:
#   --dry-run       Only update file contents, skip lock file regeneration
#                   (cargo check, sui move build). Useful for local testing.
#   --sui-tag TAG   Sui testnet release tag (e.g. testnet-v1.66.0)
#   --walrus-ref REF  Walrus git ref (40-char SHA, branch, or tag).
#                     Non-SHA refs are resolved via git ls-remote.
#
# Examples:
#   ./scripts/bump_versions.sh --sui-tag testnet-v1.66.0
#   ./scripts/bump_versions.sh --walrus-ref abc123...def
#   ./scripts/bump_versions.sh --sui-tag testnet-v1.66.0 --walrus-ref main
#   ./scripts/bump_versions.sh --dry-run --sui-tag testnet-v1.66.0

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DRY_RUN=false
SUI_TAG=""
WALRUS_REF=""

usage() {
    echo "Usage: $0 [--dry-run] [--sui-tag <tag>] [--walrus-ref <ref>]"
    echo ""
    echo "At least one of --sui-tag or --walrus-ref is required."
    echo ""
    echo "Options:"
    echo "  --dry-run         Skip lock file regeneration"
    echo "  --sui-tag TAG     Sui testnet release tag (e.g. testnet-v1.66.0)"
    echo "  --walrus-ref REF  Walrus git ref (SHA, branch, or tag)"
    echo ""
    echo "Examples:"
    echo "  $0 --sui-tag testnet-v1.66.0"
    echo "  $0 --walrus-ref abc123...def"
    echo "  $0 --sui-tag testnet-v1.66.0 --walrus-ref main"
    exit 1
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --dry-run)
            DRY_RUN=true
            shift
            ;;
        --sui-tag)
            [[ $# -lt 2 ]] && { echo "Error: --sui-tag requires a value"; usage; }
            SUI_TAG="$2"
            shift 2
            ;;
        --walrus-ref)
            [[ $# -lt 2 ]] && { echo "Error: --walrus-ref requires a value"; usage; }
            WALRUS_REF="$2"
            shift 2
            ;;
        -*)
            echo "Unknown option: $1"
            usage
            ;;
        *)
            echo "Unexpected argument: $1"
            usage
            ;;
    esac
done

# At least one flag is required.
if [[ -z "$SUI_TAG" && -z "$WALRUS_REF" ]]; then
    echo "Error: at least one of --sui-tag or --walrus-ref is required"
    usage
fi

# --- Sui bump ---
if [[ -n "$SUI_TAG" ]]; then
    # Validate tag format.
    if [[ ! "$SUI_TAG" =~ ^testnet-v[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
        echo "Error: --sui-tag must match format 'testnet-vX.Y.Z', got '$SUI_TAG'"
        exit 1
    fi

    echo "=== Bumping Sui testnet version to $SUI_TAG ==="

    # Update tag references in site-builder/Cargo.toml.
    echo "Updating site-builder/Cargo.toml (Sui tags) ..."
    sed -i -E "s|(tag = \")testnet-v[0-9]+\.[0-9]+\.[0-9]+|\1${SUI_TAG}|g" \
        "$REPO_ROOT/site-builder/Cargo.toml"

fi

# --- Walrus bump ---
if [[ -n "$WALRUS_REF" ]]; then
    if [[ "$WALRUS_REF" =~ ^[0-9a-f]{40}$ ]]; then
        # Raw SHA: use rev.
        WALRUS_KEY="rev"
        WALRUS_VAL="$WALRUS_REF"
    else
        # Verify the ref exists on the remote.
        RESOLVED=$(git ls-remote "https://github.com/MystenLabs/walrus" "$WALRUS_REF" \
            | head -1 | awk '{print $1}')
        if [[ -z "$RESOLVED" || ! "$RESOLVED" =~ ^[0-9a-f]{40}$ ]]; then
            echo "Error: could not resolve '$WALRUS_REF' on the remote"
            exit 1
        fi
        echo "Verified ref '$WALRUS_REF' resolves to $RESOLVED"
        # Tag or branch name: use tag so Cargo.toml stays human-readable.
        WALRUS_KEY="tag"
        WALRUS_VAL="$WALRUS_REF"
    fi

    echo "=== Bumping Walrus to $WALRUS_KEY = \"$WALRUS_VAL\" ==="

    # Replace whichever of rev/tag is currently used on Walrus dependency lines.
    echo "Updating site-builder/Cargo.toml (Walrus deps) ..."
    sed -i -E "/github\.com\/MystenLabs\/walrus/s#(rev|tag) = \"[^\"]+\"#${WALRUS_KEY} = \"${WALRUS_VAL}\"#g" \
        "$REPO_ROOT/site-builder/Cargo.toml"
fi

# --- Lock file regeneration ---
if [[ "$DRY_RUN" == true ]]; then
    echo "Dry run: skipping Cargo.lock regeneration (cargo check)"
    echo "Dry run: skipping Move.lock regeneration (sui move build)"
else
    echo "Regenerating Cargo.lock ..."
    # Use `cargo check` alone (no `cargo update`) so only the changed git deps
    # are re-resolved. A full `cargo update` would re-pick every transitive and
    # trip over yanked-but-already-locked crates (e.g. core2 0.4.0, pulled in
    # via mysten-network → multiaddr → multihash in the Sui dep tree).
    (cd "$REPO_ROOT" && cargo check)

    # Regenerate Move.lock files.
    while IFS= read -r move_toml; do
        move_dir="$(dirname "$move_toml")"
        echo "Regenerating Move.lock in $move_dir ..."
        (cd "$move_dir" && sui move build)
    done < <(find "$REPO_ROOT/move" -name "Move.toml" -type f)
fi

echo ""
echo "=== Done ==="
if [[ -n "$SUI_TAG" ]]; then echo "  Sui: $SUI_TAG"; fi
if [[ -n "$WALRUS_REF" ]]; then echo "  Walrus: $WALRUS_KEY = \"$WALRUS_VAL\""; fi
