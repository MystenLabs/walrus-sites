# CLAUDE.md

## Project Overview

Walrus Sites is a decentralized website hosting platform combining:
- **Walrus** (decentralized storage) for storing site files
- **Sui blockchain** for managing site metadata/ownership via Move smart contracts
- **Portals** (TypeScript/Bun) for serving sites to users over HTTPS

## Repository Structure

- `site-builder/` — Rust CLI tool for building, publishing, and managing Walrus Sites
- `portal/` — TypeScript/Bun workspace with three packages:
  - `common/` — Shared library (domain parsing, routing, BCS data, SuiNS, resource fetching)
  - `server/` — HTTP server-based portal (Redis caching, blocklist/allowlist)
  - `worker/` — Service worker-based portal (Webpack bundled)
- `move/walrus_site/` — Sui Move smart contracts (site objects, metadata, events)
- `scripts/` — Utility shell scripts (Docker portal, synthetic site generation, framework adapters)

## Build & Test Commands

### Rust (site-builder)

```bash
cargo test --all-features -- --include-ignored  # All tests (requires Walrus CLI installed)
cargo clippy --all-features -- -D warnings      # Lint (without tests)
cargo clippy --all-features --tests -- -D warnings  # Lint (with tests)
cargo fmt -- --config group_imports=StdExternalCrate,imports_granularity=Crate,imports_layout=HorizontalVertical  # Format
cargo sort -w -c                     # Check dependency sorting
cargo doc --no-deps --workspace      # Build docs (RUSTDOCFLAGS="-D warnings")
```

### Portal (TypeScript/Bun)

```bash
cd portal
bun install                          # Install all workspace deps
bun -F common test                   # Common lib tests (vitest)
bun -F server test                   # Server tests (bun test)  — run from portal/server: bun test
bun -F common coverage               # Coverage report
bun -F worker build:prod             # Build service worker
bun -F server start                  # Run server portal
bunx prettier --check --editorconfig "**/*.ts"  # Check formatting
```

### Move Smart Contracts

```bash
cd move/walrus_site
sui-debug move test --coverage       # Run tests with coverage (requires Sui CLI testnet-v1.51.3)
```

Coverage must be >= 80%.

## Code Conventions

### License Headers

All source files must include:
```
// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0
```

Enforced by `licensesnip` in CI and pre-commit.

### TypeScript Formatting

Uses `prettier` with `--editorconfig` flag. Formatting is checked from the `portal/` directory.

### PR Titles

Must follow Conventional Commits format (enforced by `amannn/action-semantic-pull-request`).

### EditorConfig

4-space indentation for most files; 2-space for YAML/TOML. Max line length 100 (150 for markdown). See `.editorconfig`.

## Architecture Notes

### Site Builder

The CLI (`site-builder/src/main.rs`) uses clap for argument parsing (`args.rs`). Core logic:
- `site/manager.rs` — orchestrates site lifecycle (create, update, delete)
- `site/builder.rs` — builds site from local files
- `site/resource.rs` — handles individual resources (files) with content-type, encoding
- `site/quilts.rs` — manages quilt storage (batching small resources into larger blobs)
- `site/contracts.rs` — Sui Move transaction building and execution
- `publish.rs` — publication workflow
- `walrus.rs` — Walrus SDK integration for blob storage
- `suins.rs` — SuiNS domain name resolution
- `config.rs` — loads `sites-config.yaml` with testnet/mainnet contexts

### Portal

The server portal (`portal/server/src/main.ts`) handles HTTP requests by:
1. Parsing the domain to extract the site name (`common` — `domain_parsing.ts`)
2. Resolving the site via SuiNS or object ID (`common` — `suins.ts`, `objectId_operations.ts`)
3. Fetching resources from Walrus via aggregator (`common` — `url_fetcher.ts`, `aggregator.ts`)
4. Decompressing and serving content (`common` — `decompress_data.ts`)
5. Handling routing for SPAs and redirects (`common` — `routing.ts`, `redirects.ts`)

The worker portal bundles the same common library into a service worker for client-side resolution.

### Move Contracts

`move/walrus_site/sources/site.move` defines the on-chain `Site` object. `metadata.move` handles resource metadata (content type, encoding, blob IDs). `events.move` defines events emitted during site operations.

## Key Dependencies

- Walrus SDK: pinned to latest testnet release tag
- Sui SDK: pinned to match Walrus's Sui dependency

## Automation

### Sui Version Bump

`scripts/bump_sui_testnet_version.sh <tag>` updates all Sui testnet version references across the repo and regenerates lock files. It can be run locally for testing.

The `.github/workflows/gen-sui-upgrade-version-pr.yml` workflow automates this: it resolves the latest tag (or accepts one as input), runs the script, and creates a PR. Triggered via `workflow_dispatch` or `repository_dispatch` from the Walrus repo when Walrus bumps its Sui dependency.

## Configuration

Site builder config (`sites-config.yaml`) supports multiple contexts (testnet, mainnet), each specifying package ID, RPC URL, wallet, and Walrus context. Also looks in `XDG_CONFIG_HOME` or `~/.config/walrus/sites-config.yaml`.
