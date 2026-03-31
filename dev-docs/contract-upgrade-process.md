# Contract Upgrade Process

## Overview

When upgrading the Move contract with new features (e.g., adding redirects support), all
dependent components (site-builder, portal) must be updated before merging to `main`. This
ensures integration testing across the full stack before the feature lands.

## Branching Strategy: Feature Integration Branch

Use a **stacked branch** approach with a feature integration branch that only merges to `main`
once all components are ready.

### Branch Hierarchy

```
main
 └── feat/<feature>                    ← integration branch (merges to main only when everything is ready)
      ├── feat/<feature>-contract      ← contract changes, reviewed & squash-merged into integration
      ├── feat/<feature>-site-builder  ← site-builder changes, branched from integration
      └── feat/<feature>-portal        ← portal changes, branched from site-builder
```

### Example: Redirects Feature

```
main
 └── feat/redirects                    ← integration branch
      ├── feat/redirects-contract      ← contract: new Redirects type, site.move functions
      │                                   (reviewed, squash-merged into feat/redirects via PR #682)
      ├── feat/redirects-site-builder  ← site-builder: redirect config, PTB building, diffing
      └── feat/redirects-portal        ← portal: serve redirects to end users
```

### Workflow

1. **Create integration branch** — branch `feat/<feature>` from `main`. This is the gathering
   point; it only merges to `main` once all components are ready.
2. **Contract changes** — develop on `feat/<feature>-contract`, get it reviewed, and
   squash-merge into the integration branch.
3. **Site-builder changes** — branch `feat/<feature>-site-builder` from the contract branch to
   start work in parallel. Once the contract branch is squash-merged into the integration
   branch, rebase onto the integration branch.
4. **Portal changes** — branch `feat/<feature>-portal` from the site-builder branch, implement
   serving logic.
5. **Merge upward** — once each layer is complete and reviewed, merge into the parent branch.
6. **Merge to `main`** — only when the full stack (contract + site-builder + portal) is complete
   and integration-tested together.

### Why Not Merge Components Individually?

- The contract upgrade changes the on-chain schema. If the site-builder or portal don't handle
  the new types, they'll break.
- Integration testing requires all components to be aware of the new contract types.
- Merging only the contract to `main` would leave `main` in a partially-updated state where
  the CLI and portal can't handle the new on-chain data.

## E2E Upgrade Workflow

The `e2e-upgrade.yml` workflow runs automatically on PRs that touch the contract, site-builder,
portal, or snake example. It verifies the upgrade path end-to-end on testnet.

When Move sources changed between the PR branch and `main`, the workflow publishes a fresh
package from `main`, upgrades it from the PR branch, and tests both old and new sites with both
portals. When there are no contract changes, it skips the publish/upgrade and uses the existing
testnet package directly.

**What it tests:**

1. Main's site-builder can publish a site and main's portal can serve it (baseline).
2. The contract upgrade from the PR branch succeeds.
3. The PR branch's site-builder can publish a site against the upgraded contract.
4. Main's portal still serves both old and new sites after the upgrade (backward compatibility).
5. The PR branch's portal serves both old and new sites (forward compatibility).

When the integration branch has all components ready, this workflow should pass. A failure
indicates that the upgrade breaks an existing site, or that a component isn't compatible with
the upgraded contract.

The workflow can also be triggered manually via `workflow_dispatch` with a `test-branch` input.

## Publishing the Upgrade

After the integration branch merges to `main`, generate the unsigned upgrade transaction for
signing:

```bash
sui client switch --env <NETWORK>
sui client upgrade \
  --sender <UPGRADE_CAP_OWNER> \
  --upgrade-capability <UPGRADE_CAP> \
  --build-env <NETWORK> \
  --serialize-unsigned-transaction \
  move/walrus_site
```

Replace:
- `<NETWORK>` — `testnet` or `mainnet`
- `<UPGRADE_CAP_OWNER>` — the address that owns the UpgradeCap
- `<UPGRADE_CAP>` — the UpgradeCap object ID (see table below)

| Network | UpgradeCap |
|---------|------------|
| testnet | `0x719b3b518ed7a2060243fbb04bcb7b635a3817cfb361f81807d551c277bdb647` |
| mainnet | `0x1cab3c76c48c023b60db0a56696d197569f006e406fb9627a8a8d1a119b1c23c` |

This produces a base64-encoded transaction that can be signed offline by the UpgradeCap owner.

## Creating the Release

After the upgrade transaction has been executed on both testnet and mainnet, follow the standard
release process. The release tag is needed for the next step (updating PackageInfo).

## Updating the PackageInfo with the new contract version

After the release is published, update the MVR `PackageInfo` objects so that the on-chain
package version points to the correct Git source. This allows MVR to resolve the source code
for the published package.

Generate the unsigned transaction:

```bash
sui client switch --env <NETWORK>
sui client ptb \
  --move-call @mvr/metadata::git::new \
    "https://github.com/MystenLabs/walrus-sites" \
    "move/walrus_site" \
    "<RELEASE_TAG>" \
  --assign git \
  --move-call @mvr/metadata::package_info::set_git_versioning \
    <PACKAGE_INFO> \
    <ON_CHAIN_VERSION> \
    git \
  --serialize-unsigned-transaction
```

Replace:
- `<NETWORK>` — `testnet` or `mainnet`
- `<RELEASE_TAG>` — the Git release tag (e.g., `mainnet-v2.8.0`)
- `<PACKAGE_INFO>` — the PackageInfo object ID (see table below)
- `<ON_CHAIN_VERSION>` — the on-chain package version number (increments with each upgrade)

| Network | PackageInfo |
|---------|-------------|
| testnet | `0x97be021af63c8b6c5e668f4d398b3a7457ff4c87cf9c347a1da3618e6a0223e4` |
| mainnet | `0x78969731e1f29f996e24261a13dd78c6a0932bc099aa02e27965bbfb1a643d86` |

This produces a base64-encoded transaction that can be signed offline by the PackageInfo owner.
