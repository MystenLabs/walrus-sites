// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { bcs, BcsType } from "@mysten/bcs";
import { fromHEX, toHEX } from "@mysten/sui/utils";
import { base64UrlSafeEncode } from "./url_safe_base64";

const Address = bcs.bytes(32).transform({
    input: (id: string) => fromHEX(id),
    output: (id) => toHEX(id),
});

// Blob IDs are represented on chain as u256, but serialized in URLs as URL-safe Base64.
const BLOB_ID = bcs.u256().transform({
    input: (id: string) => id,
    output: (id) => base64UrlSafeEncode(bcs.u256().serialize(id).toBytes()),
});

export const ResourcePathStruct = bcs.struct("ResourcePath", {
    path: bcs.string(),
});

export const ResourceStruct = bcs.struct("Resource", {
    path: bcs.string(),
    content_type: bcs.string(),
    content_encoding: bcs.string(),
    blob_id: BLOB_ID,
});

export function DynamicFieldStruct<K, V>(K: BcsType<K>, V: BcsType<V>) {
    return bcs.struct("DynamicFieldStruct<${K.name}, ${V.name}>", {
        parentId: Address,
        name: K,
        value: V,
    });
}
