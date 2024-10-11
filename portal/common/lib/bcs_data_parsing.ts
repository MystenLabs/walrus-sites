// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { bcs, BcsType } from "@mysten/bcs";
import { fromHEX, toHEX, toB64 } from "@mysten/sui/utils";
import { base64UrlSafeEncode } from "./url_safe_base64";

const Address = bcs.bytes(32).transform({
    input: (id: string) => fromHEX(id),
    output: (id) => toHEX(id),
});

// Blob IDs & hashes are represented on chain as u256, but serialized in URLs as URL-safe Base64.
const BLOB_ID = bcs.u256().transform({
    input: (id: string) => id,
    output: (id) => base64UrlSafeEncode(bcs.u256().serialize(id).toBytes()),
});

// Different than BLOB_ID, because we don't want this to be URL-safe;
// otherwise, it will mess up with the checksum results.
const DATA_HASH = bcs.u256().transform({
    input: (id: string) => id,
    output: (id) => toB64(bcs.u256().serialize(id).toBytes()),
});

export const ResourcePathStruct = bcs.struct("ResourcePath", {
    path: bcs.string(),
});

export const OPTION_U64 = bcs.option(bcs.u64()).transform({
    input: (value: number | null) => value,
    output: (value) => (value ? Number(value) : null),
});

export const RangeStruct = bcs.struct("Range", {
    start: OPTION_U64,
    end: OPTION_U64,
});

export const ResourceStruct = bcs.struct("Resource", {
    path: bcs.string(),
    headers: bcs.map(bcs.string(), bcs.string()),
    blob_id: BLOB_ID,
    blob_hash: DATA_HASH,
    range: bcs.option(RangeStruct),
});

export function DynamicFieldStruct<K, V>(K: BcsType<K>, V: BcsType<V>) {
    return bcs.struct("DynamicFieldStruct<${K.name}, ${V.name}>", {
        parentId: Address,
        name: K,
        value: V,
    });
}

export const RoutesStruct = bcs.struct("Routes", {
    routes_list: bcs.map(bcs.string(), bcs.string()),
});
