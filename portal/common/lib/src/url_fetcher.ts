// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import {
    DomainDetails,
    isResource,
    optionalRangeToHeaders as optionalRangeToRequestHeaders,
} from "@lib/types/index";
import { subdomainToObjectId } from "@lib/objectId_operations";
import { SuiNSResolver } from "@lib/suins";
import { ResourceFetcher } from "@lib/resource";
import {
    siteNotFound,
    noObjectIdFound,
    fullNodeFail,
    generateHashErrorResponse,
    resourceNotFound,
    custom404NotFound,
    aggregatorFail,
} from "@lib/http/http_error_responses";
import { blobAggregatorEndpoint, quiltAggregatorEndpoint } from "@lib/aggregator";
import { toBase64 } from "@mysten/bcs";
import { sha256 } from "@lib/crypto";
import { WalrusSitesRouter } from "@lib/routing";
import { HttpStatusCodes } from "@lib/http/http_status_codes";
import logger from "@lib/logger";
import BlocklistChecker from "@lib/blocklist_checker";
import { QuiltPatch } from "@lib/quilt";
import { instrumentationFacade } from "./instrumentation";

export const QUILT_PATCH_ID_INTERNAL_HEADER = "x-wal-quilt-patch-internal-id";
/**
* Includes all the logic for fetching the URL contents of a walrus site.
*/
export class UrlFetcher {
    constructor(
        private resourceFetcher: ResourceFetcher,
        private suinsResolver: SuiNSResolver,
        private wsRouter: WalrusSitesRouter,
        private aggregatorUrl: string,
        private b36DomainResolutionSupport: boolean
    ) { }

    /**
     * Resolves the subdomain to an object ID, and gets the corresponding resources.
     *
     * The `resolvedObjectId` variable is the object ID of the site that was previously resolved. If
     * `null`, the object ID is resolved again.
     */
    public async resolveDomainAndFetchUrl(
        parsedUrl: DomainDetails,
        resolvedObjectId: string | null,
        blocklistChecker?: BlocklistChecker
    ): Promise<Response> {
        logger.info("Resolving the subdomain to an object ID and retrieving its resources", { subdomain: parsedUrl.subdomain, path: parsedUrl.path });
        if (!resolvedObjectId) {
            const resolveObjectResult = await this.resolveObjectId(parsedUrl);
            const isObjectId = typeof resolveObjectResult == "string";
            if (!isObjectId) {
                return resolveObjectResult;
            }
            resolvedObjectId = resolveObjectResult;
        }
        instrumentationFacade.increaseRequestsMade(1, resolvedObjectId);

        if (blocklistChecker && await blocklistChecker.isBlocked(resolvedObjectId)) {
            return siteNotFound();
        }

        // Rerouting based on the contents of the routes object,
        // constructed using the ws-resource.json.

        // Initiate a fetch request to get the Routes object in case the request
        // to the initial unfiltered path fails.
        const routesPromise = this.wsRouter.getRoutes(resolvedObjectId);

        // Fetch the URL using the initial path.
        const fetchPromise = await this.fetchUrl(resolvedObjectId, parsedUrl.path);

        // If the fetch of the initial path succeeds, return the response.
        if (fetchPromise.status !== HttpStatusCodes.NOT_FOUND) {
            return fetchPromise;
        }

        // If the fetch fails, check if the path can be matched using
        // the Routes DF and fetch the redirected path.
        const routes = await routesPromise;

        if (!routes) {
            logger.warn(
                "No Routes object found for the object ID",
                { resolvedObjectIdNoRoutes: resolvedObjectId }
            );
            // Fall through to 404.html check
        }

        // Try matching route if routes exist
        if (routes) {
            const matchingRoute = this.wsRouter.matchPathToRoute(parsedUrl.path, routes);
            if (matchingRoute) {
                // If the route is found, fetch the redirected path.
                const routeResponse = await this.fetchUrl(resolvedObjectId, matchingRoute);
                if (routeResponse.status !== HttpStatusCodes.NOT_FOUND) {
                    return routeResponse;
                }
            } else {
                logger.warn(
                    `No matching route found for ${parsedUrl.path}`,
                    {
                        resolvedObjectIdNoMatchingRoute: resolvedObjectId
                    });
            }
        }

        // Try to fetch 404.html from the deployed site
        if (parsedUrl.path !== "/404.html") {
            const notFoundPage = await this.fetchUrl(resolvedObjectId, "/404.html");
            if (notFoundPage.status !== HttpStatusCodes.NOT_FOUND) {
                return notFoundPage;
            }

            // Site doesn't have its own 404 page — use portal fallback
            return custom404NotFound();
        }

        return custom404NotFound();
    }

    async resolveObjectId(
        parsedUrl: DomainDetails,
    ): Promise<string | Response> {
        logger.info("Resolving the subdomain to an object ID", { subdomain: parsedUrl.subdomain });

        // Resolve to an objectId using a hard-coded subdomain.
        const hardCodedObjectId = this.suinsResolver.hardcodedSubdomains(parsedUrl.subdomain);
        if (hardCodedObjectId) return hardCodedObjectId;

        // If b36 subdomains are supported, resolve them by converting them to a hex object id.
        const isSuiNSDomain = parsedUrl.subdomain.includes(".");
        const isb36Domain = !isSuiNSDomain;
        if (this.b36DomainResolutionSupport && isb36Domain) {
            // Try to convert the subdomain to an object ID NOTE: This effectively _disables_ any SuiNs
            // name that is the base36 encoding of an object ID (i.e., a 32-byte string). This is
            // desirable, prevents people from getting suins names that are the base36 encoding the
            // object ID of a target site (with the goal of hijacking non-suins queries).
            const resolvedB36toHex = subdomainToObjectId(parsedUrl.subdomain);
            if (resolvedB36toHex) return resolvedB36toHex;
        }

        // Resolve the SuiNS domain to an object id.
        try {
            const objectId = await this.suinsResolver.resolveSuiNsAddress(parsedUrl.subdomain);
            if (objectId) return objectId;
            logger.warn(
                "Unable to resolve the SuiNS domain. Is the domain valid?",
                { subdomain: parsedUrl.subdomain }
            )
            return noObjectIdFound();
        } catch {
            logger.error(
                "Unable to reach the full node during suins domain resolution",
                { subdomain: parsedUrl.subdomain }
            );
            return fullNodeFail();
        }
    }

    /**
     * Fetches the URL of a walrus site.
     * @param objectId - The object ID of the site object.
     * @param path - The path of the site resource to fetch. e.g. /index.html
     */
    public async fetchUrl(
        objectId: string,
        path: string,
    ): Promise<Response> {
        const result = await this.resourceFetcher.fetchResource(objectId, path, new Set<string>());
        if (!isResource(result) || !result.blob_id) {
            // TODO: #SEW-516 This gets overridden by custom404NotFound from the caller of this
            // function
            return resourceNotFound();
        }

        logger.info("Successfully fetched resource!", { fetchedResourceResult: JSON.stringify(result) });

        const quilt_patch_internal_id = result.headers.get(QUILT_PATCH_ID_INTERNAL_HEADER)
        let aggregator_endpoint: URL;
        let blobOrPatchId: string;
        if (quilt_patch_internal_id) {
            const quilt_patch = new QuiltPatch(result.blob_id, quilt_patch_internal_id)
            const quilt_patch_id = quilt_patch.derive_id()
            blobOrPatchId = quilt_patch_id;
            logger.info("Resource is stored as a quilt patch.", { quilt_patch_id })
            aggregator_endpoint = quiltAggregatorEndpoint(quilt_patch_id, this.aggregatorUrl)
        } else {
            logger.info("Resource is stored as a blob.", { blob_id: result.blob_id })
            blobOrPatchId = result.blob_id;
            aggregator_endpoint = blobAggregatorEndpoint(result.blob_id, this.aggregatorUrl)
        }

        // We have a resource, get the range header.
        let range_header = optionalRangeToRequestHeaders(result.range);
        logger.info("Fetching blob from aggregator", { aggregatorUrl: this.aggregatorUrl, blob_id: result.blob_id })

        const aggregatorFetchingStart = Date.now();
        const response = await this.fetchWithRetry(aggregator_endpoint, { headers: range_header });
        if (!response.ok) {
            if (response.status === 404) {
                // TODO: #SEW-516 This gets overridden by custom404NotFound from the caller of this
                // function
                return resourceNotFound();
            } else if (response.status >= 500 && response.status < 600) {
                logger.error(
                    "Failed to fetch resource! Response from aggregator endpoint not ok.",
                    { path: result.path, status: response.status }
                );
                return aggregatorFail();
            } else { // If we do not get one of the above, it makes sense to log it and throw
                // another error in order to investigate how we to handle this new type of response.
                let contents = await response.text();
                logger.warn("Unexpected response from aggregator.", { aggregator_endpoint, status: response.status, contents });
                // Will return genericError.
                throw new Error(`Unhandled response status from aggregator. Response status: ${response.status}`);
            }
        }
        const aggregatorFetchingDuration = Date.now() - aggregatorFetchingStart;
        instrumentationFacade.recordAggregatorTime(aggregatorFetchingDuration, { siteId: objectId, path, blobOrPatchId });

        const body = await response.arrayBuffer();
        // Verify the integrity of the aggregator response by hashing
        // the response contents.
        const h10b = toBase64(await sha256(body));
        if (result.blob_hash != h10b) {
            logger.error(
                "Checksum mismatch! The hash of the fetched resource does not " +
                "match the hash of the aggregator response.", {
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

    /**
     * Attempts to fetch a resource from the given input URL or Request object, with retry logic.
     *
     * Retries the fetch operation up to a specified number of attempts in case of failure,
     * with a delay between each retry. Logs the status and error messages during retries.
     *
     * @param input - The URL string, URL object, or Request object representing the resource to fetch.
     * @param init - Optional fetch options such as headers, method, and body.
     * @param retries - The maximum number of retry attempts (default is 3).
     * @param delayMs - The delay in milliseconds between retry attempts (default is 1000ms).
     * @returns A promise that resolves with the successful `Response` object or rejects with the last error.
     */
    private async fetchWithRetry(
        input: string | URL | globalThis.Request,
        init?: RequestInit,
        retries: number = 2,
        delayMs: number = 1000
    ): Promise<Response> {
        let lastError: unknown;

        if (retries < 0) {
            logger.warn(
                `Invalid retries value (${retries}). Falling back to a single fetch call.`
            );
            retries = 0;
        }

        for (let attempt = 0; attempt <= retries; attempt++) {
            try {
                const response = await fetch(input, init);

                if (response.status === 404) { // If 404 error, log the response status and do not retry.
                    logger.info("Aggregator responded with NOT_FOUND (404)", { input })
                } else if (response.status >= 500 && response.status < 600) {
                    if (attempt === retries) {
                        return response;
                    }
                    throw new Error(`Server responded with status ${response.status}`);
                } else if (!response.ok) { // If non-5xx error, log the response status and do not retry.
                    logger.warn("Aggregator responded with unexpected status.", { input, status: response.status });
                }

                return response;
            } catch (error) {
                logger.error(
                    "Fetch attempt failed",
                    {
                        attempt: attempt + 1,
                        totalAttempts: retries + 1,
                        error: error instanceof Error ? error.message : error,
                    });
                lastError = error;
            }

            // Wait before retrying
            if (attempt < retries) {
                await new Promise(resolve => setTimeout(resolve, delayMs));
            }
        }
        // All retry attempts failed; throw the last encountered error.
        throw lastError instanceof Error ? lastError : new Error('Unknown error occurred in fetchWithRetry');
    }
}
