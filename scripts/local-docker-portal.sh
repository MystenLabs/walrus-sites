#!/bin/bash
# Copyright (c) Mysten Labs, Inc.
# SPDX-License-Identifier: Apache-2.0

# Runs the walrus-sites portal Docker image locally for a given network and version.
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

log "running the walrus-sites portal Docker image for network: $network and version: $version"

# Base36-encoded object IDs for the default landing page served at the portal root.
get-landing-page-oid-b36() {
  case "$network" in
    mainnet) echo "46f3881sp4r55fc6pcao9t93bieeejl4vr4k2uv8u4wwyx1a93";;
    testnet) echo "1p3repujoigwcqrk0w4itsxm7hs7xjl4hwgt3t0szn6evad83q";;
    *) die "unsupported network: $network";;
  esac
}

# On-chain package address for the Walrus Sites Move package.
get-site-package() {
  case "$network" in
    mainnet) echo "0x26eb7ee8688da02c5f671679524e379f0b837a12f1d1d799f255b7eea260ad27";;
    testnet) echo "0xf99aee9f21493e1590e7e5a9aea6f343a1f381031a04a732724871fc294be799";;
    *) die "unsupported network: $network";;
  esac
}

docker run \
  -it \
  --rm \
  -e ENABLE_ALLOWLIST=false \
  -e ENABLE_BLOCKLIST=false \
  -e LANDING_PAGE_OID_B36="$(get-landing-page-oid-b36)" \
  -e PORTAL_DOMAIN_NAME_LENGTH=21 \
  -e PREMIUM_RPC_URL_LIST=https://fullnode."$network".sui.io \
  -e RPC_URL_LIST=https://fullnode."$network".sui.io \
  -e SUINS_CLIENT_NETWORK="$network" \
  -e AGGREGATOR_URL=https://aggregator.walrus-"$network".walrus.space \
  -e SITE_PACKAGE="$(get-site-package)" \
  -e B36_DOMAIN_RESOLUTION_SUPPORT=true \
  -p 3000:3000 \
  mysten/walrus-sites-server-portal:"$network"-"$version"
