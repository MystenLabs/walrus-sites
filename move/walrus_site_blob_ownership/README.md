# Walrus Sites Package

This directory contains the main walrus sites package.

You can find the latest addresses for this package and more information
[in the docs page](https://docs.wal.app/walrus-sites/intro.html).

## Overview

Walrus Sites are "web"-sites that use Sui and Walrus as their underlying technology.

They are a prime example of how Walrus can be used to build new and exciting decentralized applications.

Anyone can build and deploy a Walrus Site and make it accessible to the world!

Interestingly, this documentation is itself available as a Walrus Site at https://docs.wal.app/walrus-sites/intro.html
(if you aren't there already).

## Modules

site: Provides functionality for managing and deploying Walrus Site objects and their associated contents.

events: Defines event types and utilities for tracking and emitting events related to Walrus Site operations e.g. creation, deletion.

metadata: Defines a metadata struct related to Walrus Sites objects and exposes getters/setters for managing metadata.

## Installing

### [Move Registry CLI](https://docs.suins.io/move-registry)

```bash
mvr add @walrus/sites --network testnet

# or for mainnet
mvr add @walrus/sites --network mainnet
```
