// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { getFullnodeUrl, SuiClient } from "@mysten/sui/client";
import { NETWORK } from "@lib/constants";
import { DomainDetails, isResource } from "@lib/types/index";
import { subdomainToObjectId, HEXtoBase36 } from "@lib/objectId_operations";
import { resolveSuiNsAddress, hardcodedSubdmains } from "@lib/suins";
import { fetchResource } from "@lib/resource";
import { siteNotFound, noObjectIdFound, fullNodeFail } from "@lib/http/http_error_responses";
import { decompressData } from "@lib/decompress_data";
import { aggregatorEndpoint } from "./aggregator";

/**
 * Resolves the subdomain to an object ID, and gets the corresponding resources.
 */
export async function resolveAndFetchPage(parsedUrl: DomainDetails): Promise<Response> {
    const rpcUrl = getFullnodeUrl(NETWORK);
    const client = new SuiClient({ url: rpcUrl });

    let objectId = hardcodedSubdmains(parsedUrl.subdomain);
    if (!objectId && !parsedUrl.subdomain.includes('.')) {
        // Try to convert the subdomain to an object ID NOTE: This effectively _disables_ any SuiNs
        // name that is the base36 encoding of an object ID (i.e., a 32-byte string). This is
        // desirable, prevents people from getting suins names that are the base36 encoding the
        // object ID of a target site (with the goal of hijacking non-suins queries).
        //
        // If the subdomain contains `.`, it is a SuiNS name, and we should not convert it.
        objectId = subdomainToObjectId(parsedUrl.subdomain);
    }
    if (!objectId) {
        // Check if there is a SuiNs name
        try {
            objectId = await resolveSuiNsAddress(client, parsedUrl.subdomain);
        } catch {
            return fullNodeFail();
        }
    }
    if (objectId) {
        console.log("Object ID: ", objectId);
        console.log("Base36 version of the object ID: ", HEXtoBase36(objectId));
        return fetchPage(client, objectId, parsedUrl.path);
    }
    return noObjectIdFound();
}

/**
 * Fetches a page.
 */
async function fetchPage(client: SuiClient, objectId: string, path: string): Promise<Response> {
    const result = await fetchResource(client, objectId, path, new Set<string>);
    if (!isResource(result) || !result.blob_id) {
        if (path !== '/404.html') {
            return fetchPage(client, objectId, '/404.html');
        } else {
            return siteNotFound();
        }
    }

    console.log("Fetched Resource: ", result);
    const contents = await fetch(aggregatorEndpoint(result.blob_id));
    if (!contents.ok) {
        return siteNotFound();
    }

    // Deserialize the bcs encoded body and decompress.
    const body = await contents.arrayBuffer();
    const decompressed = await decompressData(body, result.content_encoding);
    if (!decompressed) {
        return siteNotFound();
    }
    console.log("Returning resource: ", result.path, result.blob_id, result.content_type);
    return new Response(decompressed, {
        headers: {
            "Content-Type": result.content_type,
        },
    });
}
