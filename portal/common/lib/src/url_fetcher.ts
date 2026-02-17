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
    custom404NotFound,
    aggregatorFail,
    blobUnavailable,
} from "@lib/http/http_error_responses";
import { blobAggregatorEndpoint, quiltAggregatorEndpoint } from "@lib/aggregator";
import { toBase64 } from "@mysten/bcs";
import { sha256 } from "@lib/crypto";
import { WalrusSitesRouter } from "@lib/routing";
import logger, { formatError } from "@lib/logger";
import BlocklistChecker from "@lib/blocklist_checker";
import { QuiltPatch } from "@lib/quilt";
import { instrumentationFacade } from "./instrumentation";
import { ExecuteResult, PriorityExecutor } from "@lib/priority_executor";

type AggregatorResult =
    | { type: "ok"; body: ArrayBuffer; elapsedMs: number }
    | { type: "blob_unavailable" };

export const QUILT_PATCH_ID_INTERNAL_HEADER = "x-wal-quilt-patch-internal-id";

/**
 * Discriminated union returned by `fetchUrl`.
 *
 * Uses a `status` field as the discriminator, similar to Rust enums.
 * TypeScript will narrow the type when you check `result.status`,
 * forcing callers to handle each case before accessing the response.
 *
 * - `Ok`: Successfully fetched the resource
 * - `ResourceNotFound`: The on-chain resource doesn't exist (try fallbacks)
 * - `BlobUnavailable`: The blob exists on-chain but expired on Walrus
 * - `AggregatorFail`: The aggregator is unreachable or returned a server error
 * - `HashMismatch`: The blob hash doesn't match the on-chain hash
 */
export type FetchUrlResult =
    | { status: "Ok"; response: Response }
    | { status: "ResourceNotFound" }
    | { status: "BlobUnavailable"; response: Response }
    | { status: "AggregatorFail"; response: Response }
    | { status: "HashMismatch"; response: Response };
/**
 * Includes all the logic for fetching the URL contents of a walrus site.
 */
export class UrlFetcher {
    constructor(
        private resourceFetcher: ResourceFetcher,
        private suinsResolver: SuiNSResolver,
        private wsRouter: WalrusSitesRouter,
        private aggregatorExecutor: PriorityExecutor,
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
        logger.info("Resolving the subdomain to an object ID and retrieving its resources", {
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
        instrumentationFacade.increaseRequestsMade(1, resolvedObjectId);

        if (blocklistChecker && (await blocklistChecker.isBlocked(resolvedObjectId))) {
            return siteNotFound();
        }

        // Rerouting based on the contents of the routes object,
        // constructed using the ws-resource.json.

        // Initiate a fetch request to get the Routes object in case the request
        // to the initial unfiltered path fails.
        const routesPromise = this.wsRouter.getRoutes(resolvedObjectId);

        // Fetch the URL using the initial path.
        const fetchResult = await this.fetchUrl(resolvedObjectId, parsedUrl.path);

        // Only fall through to routing/fallbacks when the on-chain resource
        // doesn't exist. Terminal errors (expired blob, aggregator failure, etc.)
        // are returned immediately.
        if (fetchResult.status !== "ResourceNotFound") {
            return fetchResult.response;
        }

        // The on-chain resource was not found — try route matching and fallbacks.
        const routes = await routesPromise;

        if (!routes) {
            logger.warn("No Routes object found for the object ID", {
                resolvedObjectIdNoRoutes: resolvedObjectId,
            });
            // Fall through to 404.html check
        }

        // Try matching route if routes exist
        if (routes) {
            const matchingRoute = this.wsRouter.matchPathToRoute(parsedUrl.path, routes);
            if (matchingRoute) {
                const routeResult = await this.fetchUrl(resolvedObjectId, matchingRoute);
                if (routeResult.status !== "ResourceNotFound") {
                    return routeResult.response;
                }
            } else {
                logger.warn(`No matching route found for ${parsedUrl.path}`, {
                    resolvedObjectIdNoMatchingRoute: resolvedObjectId,
                });
            }
        }

        // Try to fetch 404.html from the deployed site
        if (parsedUrl.path !== "/404.html") {
            const notFoundResult = await this.fetchUrl(resolvedObjectId, "/404.html");
            if (notFoundResult.status === "Ok") {
                // Success - return the site's custom 404 page
                return notFoundResult.response;
            }

            if (
                notFoundResult.status !== "ResourceNotFound" &&
                notFoundResult.status !== "BlobUnavailable"
            ) {
                // Terminal errors (aggregator failure, hash mismatch) are returned as-is.
                return notFoundResult.response;
            }

            // The site either doesn't have a 404 page, or the 404 page's blob
            // has expired — either way, use the portal's own fallback.
            return custom404NotFound();
        }

        return custom404NotFound();
    }

    async resolveObjectId(parsedUrl: DomainDetails): Promise<string | Response> {
        logger.info("Resolving the subdomain to an object ID", {
            subdomain: parsedUrl.subdomain,
        });

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
            logger.warn("Unable to resolve the SuiNS domain. Is the domain valid?", {
                subdomain: parsedUrl.subdomain,
            });
            return noObjectIdFound();
        } catch {
            logger.error("Unable to reach the full node during suins domain resolution", {
                subdomain: parsedUrl.subdomain,
            });
            return fullNodeFail();
        }
    }

    /**
     * Fetches the URL of a walrus site.
     *
     * Returns a discriminated union so that the caller can distinguish between
     * "resource doesn't exist on-chain" (eligible for fallback routing) and
     * terminal errors like expired blobs or aggregator failures.
     *
     * @param objectId - The object ID of the site object.
     * @param path - The path of the site resource to fetch. e.g. /index.html
     */
    public async fetchUrl(objectId: string, path: string): Promise<FetchUrlResult> {
        const result = await this.resourceFetcher.fetchResource(objectId, path, new Set<string>());
        if (!isResource(result) || !result.blob_id) {
            return { status: "ResourceNotFound" };
        }

        logger.info("Successfully fetched resource!", {
            fetchedResourceResult: JSON.stringify(result),
        });

        const quilt_patch_internal_id = result.headers.get(QUILT_PATCH_ID_INTERNAL_HEADER);
        let blobOrPatchId: string;
        let endpointBuilder: (aggregatorUrl: string) => URL;
        if (quilt_patch_internal_id) {
            const quilt_patch = new QuiltPatch(result.blob_id, quilt_patch_internal_id);
            const quilt_patch_id = quilt_patch.derive_id();
            blobOrPatchId = quilt_patch_id;
            logger.info("Resource is stored as a quilt patch.", { quilt_patch_id });
            endpointBuilder = (url) => quiltAggregatorEndpoint(quilt_patch_id, url);
        } else {
            logger.info("Resource is stored as a blob.", { blob_id: result.blob_id });
            blobOrPatchId = result.blob_id;
            endpointBuilder = (url) => blobAggregatorEndpoint(result.blob_id, url);
        }

        // We have a resource, get the range header.
        const range_header = optionalRangeToRequestHeaders(result.range);
        logger.info("Fetching blob from aggregator", { blob_id: result.blob_id });

        // Use priority executor for aggregator fallback
        let aggregatorResult: AggregatorResult;
        try {
            aggregatorResult = await this.aggregatorExecutor.invoke(
                async (aggregatorUrl): Promise<ExecuteResult<AggregatorResult>> => {
                    const endpoint = endpointBuilder(aggregatorUrl);
                    logger.debug("Trying aggregator", {
                        aggregatorUrl,
                        endpoint: endpoint.toString(),
                    });
                    return this.tryAggregator(endpoint, range_header);
                },
            );
        } catch (error) {
            // All aggregators failed (exhausted retries or stopped)
            logger.error("All aggregators failed", {
                error: formatError(error),
                path,
                blobOrPatchId,
            });
            return {
                status: "AggregatorFail",
                response: aggregatorFail(),
            };
        }

        // Handle semantic result
        if (aggregatorResult.type === "blob_unavailable") {
            return {
                status: "BlobUnavailable",
                response: blobUnavailable(blobOrPatchId),
            };
        }

        instrumentationFacade.recordAggregatorTime(aggregatorResult.elapsedMs, {
            siteId: objectId,
            path,
            blobOrPatchId,
        });

        const body = aggregatorResult.body;
        // Verify the integrity of the aggregator response by hashing
        // the response contents.
        const h10b = toBase64(await sha256(body));
        if (result.blob_hash != h10b) {
            logger.error(
                "Checksum mismatch! The hash of the fetched resource does not " +
                    "match the hash of the aggregator response.",
                {
                    path: result.path,
                    blobHash: result.blob_hash,
                    aggrHash: h10b,
                },
            );
            return {
                status: "HashMismatch",
                response: generateHashErrorResponse(),
            };
        }

        return {
            status: "Ok",
            response: new Response(body, {
                status: path === "/404.html" ? 404 : 200,
                headers: {
                    ...Object.fromEntries(result.headers),
                    "x-resource-sui-object-version": result.version,
                    "x-resource-sui-object-id": result.objectId,
                    "x-unix-time-cached": Date.now().toString(),
                },
            }),
        };
    }

    private async tryAggregator(
        url: URL,
        headers: { [key: string]: string },
    ): Promise<ExecuteResult<AggregatorResult>> {
        const start = Date.now();

        try {
            const response = await fetch(url, { headers });

            if (response.ok) {
                const body = await response.arrayBuffer();
                return {
                    status: "success",
                    value: { type: "ok", body, elapsedMs: Date.now() - start },
                };
            }

            // Aggregator error codes (from aggregator_openapi.yaml):
            // 403: Blob size exceeds maximum allowed size configured for this aggregator.
            // 404: Blob not stored on Walrus (likely expired) or quilt patch doesn't exist.
            // 416: Invalid byte range parameters (would indicate a bug since ranges come from on-chain data).
            // 5xx: Internal server error.

            if (response.status === 404) {
                logger.warn("Blob not available on aggregator (likely expired)", {
                    url: `${url.origin}${url.pathname}`,
                });
                return { status: "success", value: { type: "blob_unavailable" } };
            }

            if (response.status === 403) {
                // Blob size exceeds this aggregator's configured max — try next aggregator
                // which may have a higher limit.
                logger.error("Aggregator rejected blob due to size limit", {
                    url: `${url.origin}${url.pathname}`,
                });
                return {
                    status: "retry-next",
                    error: new Error("Aggregator returned 403 (size limit)"),
                };
            }

            if (response.status === 502) {
                logger.warn("Aggregator 502, trying next", { url: `${url.origin}${url.pathname}` });
                return {
                    status: "retry-next",
                    error: new Error(`Aggregator returned 502`),
                };
            }

            if (response.status >= 500) {
                logger.warn("Aggregator 5xx, retrying", {
                    url: `${url.origin}${url.pathname}`,
                    status: response.status,
                });
                return {
                    status: "retry-same",
                    error: new Error(`Aggregator returned ${response.status}`),
                };
            }

            // 4xx (except 404) - client error, stop
            logger.error("Aggregator client error", {
                url: `${url.origin}${url.pathname}`,
                status: response.status,
            });
            return {
                status: "stop",
                error: new Error(`Aggregator client error: ${response.status}`),
            };
        } catch (error) {
            // Network error (connection refused, timeout, etc.)
            logger.warn("Aggregator network error", {
                url: `${url.origin}${url.pathname}`,
                error: formatError(error),
            });
            return {
                status: "retry-next",
                error: new Error("Aggregator network error", { cause: error }),
            };
        }
    }
}
