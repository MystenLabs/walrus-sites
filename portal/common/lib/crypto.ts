// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

const { subtle } = globalThis.crypto;

/**
 * Calculates SHA-256 hash of input message.
 * @param message ArrayBuffer to hash
 * @returns Promise<Uint8Array> Resulting hash as Uint8Array
 */
export async function sha256(message: ArrayBuffer): Promise<Uint8Array> {
    const hash = await subtle.digest("SHA-256", message);
    return new Uint8Array(hash);
}
