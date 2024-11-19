// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

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
import logger from "./logger";

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
    logger.info({ message: "parsed-url", subdomain: parsedUrl.subdomain, path: parsedUrl.path });
    if (!resolvedObjectId) {
        const resolveObjectResult = await resolveObjectId(parsedUrl);
        const isObjectId = typeof resolveObjectResult == "string";
        if (!isObjectId) {
            return resolveObjectResult;
        }
        resolvedObjectId = resolveObjectResult;
    }

    logger.info({ message: "Resolved object id", resolvedObjectId: resolvedObjectId });
    logger.info({ message: "Base36 version of the object id", base36OfObjectId: HEXtoBase36(resolvedObjectId) });
    // Rerouting based on the contents of the routes object,
    // constructed using the ws-resource.json.

    // Initiate a fetch request to get the Routes object in case the request
    // to the initial unfiltered path fails.
    const routesPromise = getRoutes(resolvedObjectId);

    // Fetch the page using the initial path.
    const fetchPromise = await fetchPage(resolvedObjectId, parsedUrl.path);

    // If the fetch fails, check if the path can be matched using
    // the Routes DF and fetch the redirected path.
    if (fetchPromise.status == HttpStatusCodes.NOT_FOUND) {
        const routes = await routesPromise;
        if (!routes) {
            logger.warn({
                message: "No routes found for the object ID",
                resolvedObjectIdNoRoutes: resolvedObjectId
            });
            return siteNotFound();
        }
        let matchingRoute: string | undefined;
        matchingRoute = matchPathToRoute(parsedUrl.path, routes);
        if (!matchingRoute) {
            logger.warn({
                message: `No matching route found for ${parsedUrl.path}`,
                resolvedObjectIdNoMatchingRoute: resolvedObjectId
            });
            return siteNotFound();
        }
        return fetchPage(resolvedObjectId, matchingRoute);
    }
    return fetchPromise;
}

export async function resolveObjectId(
    parsedUrl: DomainDetails,
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
            // TODO: only check for SuiNs names if the subdomain is not a valid base36 string.
            objectId = await resolveSuiNsAddress(parsedUrl.subdomain);
            if (!objectId) {
                logger.warn({
                    message: "Could not resolve SuiNs domain. Does the domain exist?",
                    subdomain: parsedUrl.subdomain,
                })
                return noObjectIdFound();
            }
            return objectId;
        } catch {
            logger.error({
                message: "Failed to contact the full node while resolving suins domain",
                subdomain: parsedUrl.subdomain
            });
            return fullNodeFail();
        }
    }
    return objectId;
}

/**
 * Fetches a page.
 */
export async function fetchPage(
    objectId: string,
    path: string,
): Promise<Response> {
    logger.info({message: 'Fetching page', objectId: objectId, path: path});
    const result = await fetchResource(objectId, path, new Set<string>());
    if (!isResource(result) || !result.blob_id) {
        if (path !== "/404.html") {
            logger.warn({ message: "Resource not found. Fetching /404.html ...", path });
            return fetchPage(objectId, "/404.html");
        } else {
            logger.warn({ message: "Failed to fetch resource! Does this page exist?", path });
            return siteNotFound();
        }
    }

    logger.info({ message: "Successfully fetched resource!", fetchedResourceResult: JSON.stringify(result) });

    // We have a resource, get the range header.
    logger.info({ message: "Add the range headers of the resource", range: JSON.stringify(result.range)});
    let range_header = optionalRangeToRequestHeaders(result.range);
    const contents = await fetch(aggregatorEndpoint(result.blob_id), { headers: range_header });
    if (!contents.ok) {
        logger.error(
            {
                message: "Failed to fetch resource! Response from aggregator endpoint not ok.",
                path: result.path,
                status: contents.status
            });
        return siteNotFound();
    }

    const body = await contents.arrayBuffer();
    // Verify the integrity of the aggregator response by hashing
    // the response contents.
    const h10b = toBase64(await sha256(body));
    if (result.blob_hash != h10b) {
        logger.error({
            message: "Checksum mismatch! The hash of the fetched resource does not " +
            "match the hash of the aggregator response.",
            path: result.path,
            blobHash: result.blob_hash,
            aggrHash: h10b
        });
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
