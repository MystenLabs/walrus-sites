// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

// TODO: These functions are copy-pasted from the portal/common/ library.
// At some point we will have to mvoe the common library inside the ts-sdk.

const { subtle } = globalThis.crypto

/**
 * Calculates SHA-256 hash of input message.
 * @param message ArrayBuffer to hash
 * @returns Promise<Uint8Array> Resulting hash as Uint8Array
 */
export async function sha256(message: Buffer): Promise<Uint8Array> {
    const hash = await subtle.digest('SHA-256', message)
    return new Uint8Array(hash)
}

export function toQuiltPatchIdHex(patch_id: string): string {
    const identifier_buffer = Buffer.from(patch_id, 'base64')
    const dv = new DataView(identifier_buffer.buffer, identifier_buffer.byteOffset)
    const internal_id_buf = Buffer.alloc(5)
    const dv_internal_id = new DataView(internal_id_buf.buffer, internal_id_buf.byteOffset)
    dv_internal_id.setUint8(0, dv.getUint16(32, true))
    dv_internal_id.setUint16(1, dv.getUint16(33, true), true)
    dv_internal_id.setUint16(3, dv.getUint16(35, true), true)
    const hex = bufferToHex(dv_internal_id.buffer)
    return '0x' + hex
}

function bufferToHex(buffer: ArrayBuffer) {
    const uint8Array = new Uint8Array(buffer)
    return Array.from(uint8Array)
        .map((b) => b.toString(16).padStart(2, '0'))
        .join('')
}

export const QUILT_PATCH_ID_INTERNAL_HEADER = 'x-wal-quilt-patch-internal-id'
