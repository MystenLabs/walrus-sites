// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

// Monther of all errors. Every Walrus-Sites-related error should extend this.
export class WalrusSitesClientError extends Error {}

export class MissingRequiredWalrusClient extends WalrusSitesClientError {}
export class NotImplemented extends WalrusSitesClientError {}
