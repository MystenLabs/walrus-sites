// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import {
    DomainDetails,
    isResource,
    optionalRangeToHeaders as optionalRangeToRequestHeaders,
    Routes,
} from "./types/index";
import { subdomainToObjectId, HEXtoBase36 } from "./objectId_operations";
import { SuiNSResolver } from "./suins";
import { ResourceFetcher } from "./resource";
import {
    siteNotFound,
    noObjectIdFound,
    fullNodeFail,
    generateHashErrorResponse,
    resourceNotFound,
} from "./http/http_error_responses";
import { aggregatorEndpoint } from "./aggregator";
import { toBase64 } from "@mysten/bcs";
import { sha256 } from "./crypto";
import { WalrusSitesRouter } from "./routing";
import { HttpStatusCodes } from "./http/http_status_codes";
import logger from "./logger";
import BlocklistChecker from "./blocklist_checker";
import { instrumentationFacade } from "./instrumentation";

/**
 * Includes all the logic for fetching the URL contents of a walrus site.
 */
export class UrlFetcher {
    constructor(
        private resourceFetcher: ResourceFetcher,
        private suinsResolver: SuiNSResolver,
        private wsRouter: WalrusSitesRouter,
        private aggregatorUrl: string,
        private b36DomainResolutionSupport: boolean,
    ) {}

    /**
     * Resolves the subdomain to an object ID, and gets the corresponding resources.
     *
     * The `resolvedObjectId` variable is the object ID of the site that was previously resolved. If
     * `null`, the object ID is resolved again.
     */
    public async resolveDomainAndFetchUrl(
        parsedUrl: DomainDetails,
        resolvedObjectId: string | null,
        blocklistChecker?: BlocklistChecker,
    ): Promise<Response> {
        const reqStartTime = Date.now();

        logger.debug({
            message: "parsed-url",
            subdomain: parsedUrl.subdomain,
            path: parsedUrl.path,
        });
        if (!resolvedObjectId) {
            const resolveObjectResult = await this.resolveObjectId(parsedUrl);
            const isObjectId = typeof resolveObjectResult == "string";
            if (!isObjectId) {
                return resolveObjectResult;
            }
            resolvedObjectId = resolveObjectResult;
        }

        logger.debug({ message: "Resolved object id", resolvedObjectId: resolvedObjectId });
        logger.debug({
            message: "Base36 version of the object id",
            base36OfObjectId: HEXtoBase36(resolvedObjectId),
        });
        if (blocklistChecker && (await blocklistChecker.isBlocked(resolvedObjectId))) {
            instrumentationFacade.bumpBlockedRequests();
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
            logger.warn({
                message: "No routes found for the object ID",
                resolvedObjectIdNoRoutes: resolvedObjectId,
            });
            // Fall through to 404.html check
        }

        // Try matching route if routes exist
        if (routes) {
            const matchingRoute = this.wsRouter.matchPathToRoute(parsedUrl.path, routes);
            if (matchingRoute) {
                // If the route is found, fetch the redirected path.
                const routeResponse = await this.fetchUrl(resolvedObjectId, matchingRoute);
                if (routeResponse.status !== HttpStatusCodes.NOT_FOUND) {
                    const routeResponseDuration = Date.now() - reqStartTime;
                    instrumentationFacade.recordResolveDomainAndFetchUrlResponseTime(
                        routeResponseDuration,
                        resolvedObjectId,
                    );
                    return routeResponse;
                }
            } else {
                logger.warn({
                    message: `No matching route found for ${parsedUrl.path}`,
                    resolvedObjectIdNoMatchingRoute: resolvedObjectId,
                });
            }
        }

        // Try to fetch 404.html
        if (parsedUrl.path !== "/404.html") {
            const notFoundPage = await this.fetchUrl(resolvedObjectId, "/404.html");
            if (notFoundPage.status !== HttpStatusCodes.NOT_FOUND) {
                return notFoundPage;
            }
        }

        instrumentationFacade.bumpSiteNotFoundRequests();
        return siteNotFound();
    }

    async resolveObjectId(parsedUrl: DomainDetails): Promise<string | Response> {
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
            logger.warn({
                message: "Could not resolve SuiNs domain. Does the domain exist?",
                subdomain: parsedUrl.subdomain,
            });
            instrumentationFacade.bumpNoObjectIdFoundRequests();
            return noObjectIdFound();
        } catch {
            logger.error({
                message: "Failed to contact the full node while resolving suins domain",
                subdomain: parsedUrl.subdomain,
            });
            instrumentationFacade.bumpFullNodeFailRequests();
            return fullNodeFail();
        }
    }

    /**
     * Fetches the URL of a walrus site.
     * @param objectId - The object ID of the site object.
     * @param path - The path of the site resource to fetch. e.g. /index.html
     */
    public async fetchUrl(objectId: string, path: string): Promise<Response> {
        logger.info({ message: "Fetching URL", objectId: objectId, path: path });
        const result = await this.resourceFetcher.fetchResource(objectId, path, new Set<string>());
        if (!isResource(result) || !result.blob_id) {
            return resourceNotFound();
        }

        logger.info({
            message: "Successfully fetched resource!",
            fetchedResourceResult: JSON.stringify(result),
        });

        // We have a resource, get the range header.
        logger.info({
            message: "Add the range headers of the resource",
            range: JSON.stringify(result.range),
        });
        let range_header = optionalRangeToRequestHeaders(result.range);
        const contents = await this.fetchWithRetry(
            aggregatorEndpoint(result.blob_id, this.aggregatorUrl),
            { headers: range_header },
        );
        if (!contents.ok) {
            logger.error({
                message: "Failed to fetch resource! Response from aggregator endpoint not ok.",
                path: result.path,
                status: contents.status,
            });
            instrumentationFacade.bumpSiteNotFoundRequests();
            return siteNotFound();
        }

        const body = await contents.arrayBuffer();
        // Verify the integrity of the aggregator response by hashing
        // the response contents.
        const h10b = toBase64(await sha256(body));
        if (result.blob_hash != h10b) {
            logger.error({
                message:
                    "Checksum mismatch! The hash of the fetched resource does not " +
                    "match the hash of the aggregator response.",
                path: result.path,
                blobHash: result.blob_hash,
                aggrHash: h10b,
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
        delayMs: number = 1000,
    ): Promise<Response> {
        let lastError: unknown;

        if (retries < 0) {
            logger.warn({
                message: `Invalid retries value (${retries}). Falling back to a single fetch call.`,
            });
            return fetch(input, init);
        }

        for (let attempt = 0; attempt <= retries; attempt++) {
            try {
                const response = await fetch(input, init);
                if (response.status === 500) {
                    if (attempt === retries) {
                        return response;
                    }
                    throw new Error("Server responded with status 500");
                }
                return response;
            } catch (error) {
                logger.error({
                    message: "Fetch attempt failed",
                    attempt: attempt + 1,
                    totalAttempts: retries + 1,
                    error: error instanceof Error ? error.message : error,
                });
                lastError = error;
            }

            // Wait before retrying
            if (attempt < retries) {
                await new Promise((resolve) => setTimeout(resolve, delayMs));
            }
        }
        // All retry attempts failed; throw the last encountered error.
        throw lastError instanceof Error
            ? lastError
            : new Error("Unknown error occurred in fetchWithRetry");
    }
}
