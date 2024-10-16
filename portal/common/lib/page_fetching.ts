// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { getFullnodeUrl, SuiClient } from "@mysten/sui/client";
import { NETWORK } from "./constants";
import {
    DomainDetails,
    isResource,
    optionalRangeToHeaders as optionalRangeToRequestHeaders,
} from "./types/index";
import { subdomainToObjectId, HEXtoBase36 } from "./objectId_operations";
import { resolveSuiNsAddress, hardcodedSubdmains } from "./suins";
import { fetchResource } from "./resource";
import {
    siteNotFound,
    noObjectIdFound,
    fullNodeFail,
    generateHashErrorResponse,
} from "./http/http_error_responses";
import { aggregatorEndpoint } from "./aggregator";
import { toBase64 } from "@mysten/bcs";
import { sha256 } from "./crypto";
import { getRoutes, matchPathToRoute } from "./routing";
import { HttpStatusCodes } from "./http/http_status_codes";

/**
 * Resolves the subdomain to an object ID, and gets the corresponding resources.
 *
 * The `resolvedObjectId` variable is the object ID of the site that was previously resolved. If
 * `null`, the object ID is resolved again.
 */
export async function resolveAndFetchPage(
    parsedUrl: DomainDetails,
    resolvedObjectId: string | null,
): Promise<Response> {
    const rpcUrl = getFullnodeUrl(NETWORK);
    const client = new SuiClient({ url: rpcUrl });

    if (!resolvedObjectId) {
        const resolveObjectResult = await resolveObjectId(parsedUrl, client);
        const isObjectId = typeof resolveObjectResult == "string";
        if (!isObjectId) {
            return resolveObjectResult;
        }
        resolvedObjectId = resolveObjectResult;
    }

    console.log("Object ID: ", resolvedObjectId);
    console.log("Base36 version of the object ID: ", HEXtoBase36(resolvedObjectId));
    // Rerouting based on the contents of the routes object,
    // constructed using the ws-resource.json.

    // Initiate a fetch request to get the Routes object in case the request
    // to the initial unfiltered path fails.
    const routesPromise = getRoutes(client, resolvedObjectId);

    // Fetch the page using the initial path.
    const fetchPromise = await fetchPage(client, resolvedObjectId, parsedUrl.path);

    // If the fetch fails, check if the path can be matched using
    // the Routes DF and fetch the redirected path.
    if (fetchPromise.status == HttpStatusCodes.NOT_FOUND) {
        const routes = await routesPromise;
        if (!routes) {
            console.warn("No routes found for the object ID");
            return siteNotFound();
        }
        let matchingRoute: string | undefined;
        matchingRoute = matchPathToRoute(parsedUrl.path, routes);
        if (!matchingRoute) {
            console.warn(`No matching route found for ${parsedUrl.path}`);
            return siteNotFound();
        }
        return fetchPage(client, resolvedObjectId, matchingRoute);
    }
    return fetchPromise;
}

export async function resolveObjectId(
    parsedUrl: DomainDetails,
    client: SuiClient,
): Promise<string | Response> {
    let objectId = hardcodedSubdmains(parsedUrl.subdomain);
    if (!objectId && !parsedUrl.subdomain.includes(".")) {
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
            if (!objectId) {
                return noObjectIdFound();
            }
            return objectId;
        } catch {
            return fullNodeFail();
        }
    }
    return objectId;
}

/**
 * Fetches a page.
 */
export async function fetchPage(
    client: SuiClient,
    objectId: string,
    path: string,
): Promise<Response> {
    const result = await fetchResource(client, objectId, path, new Set<string>());
    if (!isResource(result) || !result.blob_id) {
        if (path !== "/404.html") {
            return fetchPage(client, objectId, "/404.html");
        } else {
            return siteNotFound();
        }
    }

    console.log("Fetched Resource: ", result);

    // We have a resource, get the range header.
    let range_header = optionalRangeToRequestHeaders(result.range);
    const contents = await fetch(aggregatorEndpoint(result.blob_id), { headers: range_header });

    if (!contents.ok) {
        return siteNotFound();
    }

    const body = await contents.arrayBuffer();
    // Verify the integrity of the aggregator response by hashing
    // the response contents.
    const h10b = toBase64(await sha256(body));
    if (result.blob_hash != h10b) {
        console.warn(
            "[!] checksum mismatch [!] for:",
            result.path,
            ".",
            `blob hash: ${result.blob_hash} | aggr. hash: ${h10b}`,
        );
        return generateHashErrorResponse();
    }

    return new Response(body, {
        headers: {
            ...Object.fromEntries(result.headers),
            "x-resource-sui-object-version": result.version,
            "x-resource-sui-object-id": result.objectId,
            "x-unix-time-cached": Date.now().toString(),
        },
    });
}
