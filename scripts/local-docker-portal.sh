#!/bin/bash
# Copyright (c) Mysten Labs, Inc.
# SPDX-License-Identifier: Apache-2.0

# Runs the walrus-sites portal Docker image locally for a given network, using the
# version of the locally installed site-builder. An optional base36-encoded object ID
# can be provided to override the default landing page.
#
# Usage: local-docker-portal.sh <network> [landing-page-oid-b36]
set -euo pipefail

log() {
  printf "%s: info: %s\n" "$0" "$*"
}

die() {
  printf "%s: error: %s\n" "$0" "$*" >&2
  exit 1
}

command -v site-builder > /dev/null || die "site-builder not found in PATH"

version="$(site-builder -V | awk '{ print $2 }' | awk -F - '{ printf("v%s\n", $1) }')"
log "site-builder ($(command -v site-builder)) version is $version"

network="${1:?error: missing required positional argument: network (mainnet or testnet)}"
shift

# Default base36-encoded object IDs for the landing page served at the portal root.
default-landing-page-oid-b36() {
  case "$network" in
    mainnet) echo "46f3881sp4r55fc6pcao9t93bieeejl4vr4k2uv8u4wwyx1a93";;
    testnet) echo "1p3repujoigwcqrk0w4itsxm7hs7xjl4hwgt3t0szn6evad83q";;
    *) die "unsupported network: $network";;
  esac
}

landing_page_oid_b36="${1:-$(default-landing-page-oid-b36)}"
shift || true

# On-chain package address for the Walrus Sites Move package.
get-site-package() {
  case "$network" in
    mainnet) echo "0x26eb7ee8688da02c5f671679524e379f0b837a12f1d1d799f255b7eea260ad27";;
    testnet) echo "0xf99aee9f21493e1590e7e5a9aea6f343a1f381031a04a732724871fc294be799";;
    *) die "unsupported network: $network";;
  esac
}

log "running the walrus-sites portal Docker image for network: $network and version: $version"
log "landing page oid (b36): $landing_page_oid_b36"

# The v2.4.0 Docker image predates the YAML config migration and only reads env vars.
# Pass configuration as SCREAMING_SNAKE_CASE environment variables.
# URL list format: URL|RETRIES|METRIC (comma-separated for multiple entries).

docker run \
  --rm \
  -e SUINS_CLIENT_NETWORK="$network" \
  -e SITE_PACKAGE="$(get-site-package)" \
  -e LANDING_PAGE_OID_B36="$landing_page_oid_b36" \
  -e ENABLE_ALLOWLIST=false \
  -e ENABLE_BLOCKLIST=false \
  -e B36_DOMAIN_RESOLUTION_SUPPORT=true \
  -e RPC_URL_LIST="https://fullnode.$network.sui.io:443" \
  -e PREMIUM_RPC_URL_LIST="https://fullnode.$network.sui.io:443" \
  -e AGGREGATOR_URL="https://aggregator.walrus-$network.walrus.space" \
  -p 3000:3000 \
  mysten/walrus-sites-server-portal:mainnet-"$version"
