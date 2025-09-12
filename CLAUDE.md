# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Walrus Sites is a decentralized website hosting system built on the Sui blockchain and Walrus decentralized storage network. The project consists of three main components:

- **site-builder**: A Rust CLI tool for creating and publishing Walrus Sites from standard web builds
- **portal**: TypeScript/JavaScript implementations for accessing Walrus Sites (server and service-worker variants)
- **move**: Smart contracts for managing site metadata and ownership on Sui blockchain

## Development Commands

### Site Builder (Rust)
```bash
# Build the site-builder CLI
cargo build --package site-builder

# Run site-builder tests
cargo test --package site-builder

# Build and publish a site (example)
cargo run --package site-builder -- publish <directory>

# Check site status
cargo run --package site-builder -- list
```

### Portal Development (JavaScript/TypeScript with Bun)
```bash
# Install dependencies (from portal/ directory)
bun install

# Run server portal in development mode
bun -F server start

# Build worker portal for production
bun -F worker build:prod

# Run worker portal in development
bun -F worker serve:dev

# Run tests for common library
bun -F common test

# Run test coverage
bun -F common coverage
```

### Linting and Formatting
```bash
# Format Rust code
cargo fmt --all -- --config group_imports=StdExternalCrate,imports_granularity=Crate,imports_layout=HorizontalVertical

# Run Rust linting
cargo clippy --all-features -- -D warnings

# Run Rust linting with tests
cargo clippy --all-features --tests -- -D warnings

# Check Rust code
cargo check
```

### Pre-commit Hooks
This project uses pre-commit hooks. Run all checks:
```bash
pre-commit run --all-files
```

## Architecture

### Site Builder Architecture
The site-builder is organized into several key modules:

- `args.rs`: Command-line argument parsing and validation
- `publish.rs`: Core publishing logic, handles Walrus storage and Sui transactions
- `site/`: Site management, resource handling, and configuration
- `walrus.rs`: Walrus storage integration and blob management
- `util.rs`: Shared utilities and helper functions
- `types.rs`: Core type definitions for sites and resources
- `preprocessor.rs`: Asset preprocessing before upload

Key publishing workflow:
1. Parse and validate site directory structure
2. Preprocess assets (compression, optimization)
3. Upload resources to Walrus storage network
4. Create/update Sui smart contract with site metadata
5. Generate site summary and configuration

### Portal Architecture
The portal system has two implementations sharing common code:

**Common Library** (`portal/common/`):
- Shared utilities for Sui blockchain interaction
- Resource fetching and caching logic
- Domain parsing and SuiNS resolution
- Walrus blob retrieval

**Server Portal** (`portal/server/`):
- HTTP server implementation using Bun runtime
- Request routing and domain resolution
- Caching with Redis support
- Analytics and monitoring integration

**Worker Portal** (`portal/worker/`):
- Service Worker implementation for client-side execution
- Webpack-based build system
- In-browser resource fetching and caching

Both portals resolve domains using the pattern: `<domain>.wal.app` where `<domain>` corresponds to a SuiNS domain registered for the site.

### Smart Contract Integration
Sites are managed through Sui Move contracts located in `move/walrus_site/`:
- Site ownership and metadata management
- Version control for site updates
- Resource mapping and validation

## Configuration Files

- `sites-config.yaml`: Main configuration for site deployment targets
- `Cargo.toml`: Rust workspace configuration with Sui/Walrus dependencies
- `rust-toolchain.toml`: Specifies Rust version (1.88) and components

## Testing

### Rust Tests
```bash
# Run all Rust tests
cargo test

# Run specific test file
cargo test --test <test_name>

# Run tests with output
cargo test -- --nocapture
```

### JavaScript/TypeScript Tests
```bash
# Run common library tests
cd portal && bun -F common test

# Run with coverage
cd portal && bun -F common coverage

# Run specific test
cd portal && bun -F common test <test_file>
```

## Key Dependencies

**Rust (site-builder)**:
- `sui-sdk`, `sui-types`: Sui blockchain integration
- `walrus-core`: Walrus storage network client
- `clap`: Command-line interface
- `tokio`: Async runtime
- `anyhow`: Error handling

**JavaScript/TypeScript (portal)**:
- `@mysten/sui`: Sui SDK for JavaScript
- `@mysten/suins`: SuiNS domain resolution
- `bun`: Runtime and package manager
- `vitest`: Testing framework
- `webpack`: Bundling (worker portal)

## Development Notes

- The project uses Bun as the JavaScript runtime and package manager for portal components
- Rust toolchain is pinned to version 1.88 with clippy and rustfmt components
- Pre-commit hooks enforce code quality and formatting standards
- The workspace structure separates concerns between CLI tooling (Rust) and web services (TypeScript)
- Site resources are stored on Walrus with metadata managed by Sui smart contracts
