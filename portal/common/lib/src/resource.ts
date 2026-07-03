// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { HttpStatusCodes } from "@lib/http/http_status_codes";
import { Resource, VersionedResource } from "@lib/types";
import { MAX_REDIRECT_DEPTH } from "@lib/constants";
import { checkRedirect } from "@lib/redirects";
import { ResourcePathStruct, DynamicFieldStruct, ResourceStruct } from "./bcs_data_parsing";
import { deriveDynamicFieldID } from "@mysten/sui/utils";
import { bcs } from "@mysten/bcs";
import { RPCSelector } from "./rpc_selector";
import { SuiClientTypes } from "@mysten/sui/client";
import logger from "./logger";

/**
 * The ResourceFetcher class is responsible for fetching resources associated with a site.
 * It handles potential redirects and ensures that resources are fetched recursively up to a maximum depth.
 *
 * @class ResourceFetcher
 * @param {RPCSelector} rpcSelector - An instance of RPCSelector to interact with the Sui network.
 * @param {string} sitePackage - The package name of the site.
 */
export class ResourceFetcher {
    /// The string representing the ResourcePath struct in the walrus_site package.
    private readonly resourcePathMoveType: string;
    constructor(
        private rpcSelector: RPCSelector,
        originalPackageId: string,
    ) {
        this.resourcePathMoveType = originalPackageId + "::site::ResourcePath";
    }

    /**
     * Fetches a resource of a site.
     *
     * This function is recursive, as it will follow the special redirect field if it is set. A site can
     * have a special redirect field that points to another site, where the resources to display the
     * site are found.
     *
     * This is useful to create many objects with an associated site (e.g., NFTs), without having to
     * repeat the same resources for each object, and allowing to keep some control over the site (for
     * example, the creator can still edit the site even if the NFT is owned by someone else).
     *
     * See the `checkRedirect` function for more details.
     * To prevent infinite loops, the recursion depth is of this function is capped to
     * `MAX_REDIRECT_DEPTH`.
     *
     * Infinite loops can also be prevented by checking if the resource has already been seen.
     * This is done by using the `seenResources` set.
     */
    async fetchResource(
        objectId: string,
        path: string,
        seenResources: Set<string>,
        depth: number = 0,
    ): Promise<VersionedResource | HttpStatusCodes> {
        logger.info("Fetching resource", { objectId, path });
        const error = this.checkRedirectLimits(objectId, seenResources, depth);
        if (error) return error;

        // The dynamic field object ID can be derived, without
        // making a request to the network.
        const dynamicFieldId = deriveDynamicFieldID(
            objectId,
            this.resourcePathMoveType,
            bcs.string().serialize(path).toBytes(),
        );

        const [primaryObjectResponse, dynamicFieldResponse] = await this.fetchObjectPairData(
            objectId,
            dynamicFieldId,
        );

        seenResources.add(objectId);

        // Note: if the site object was deleted, its dynamic fields survive on
        // chain as orphans and stay fetchable by derived ID — so the resource
        // fetch below can still succeed and serve a deleted site's leftovers.

        // A miss (the site object wasn't found) carries no Display to redirect on.
        if (!(primaryObjectResponse instanceof Error)) {
            const redirectId = checkRedirect(primaryObjectResponse);
            if (redirectId) {
                return this.fetchResource(redirectId, path, seenResources, depth + 1);
            }
        } else {
            // Without this, a deleted/nonexistent site is indistinguishable in
            // the logs from a missing path (the miss below reads as a path 404).
            logger.info("Site object not found", {
                objectId,
                reason: primaryObjectResponse.message,
            });
        }

        if (dynamicFieldResponse instanceof Error) {
            // A missing dynamic field just means the requested path isn't a
            // resource of this site — a normal 404, not an error.
            logger.info("No resource found for dynamic field object", {
                dynamicFieldId,
                reason: dynamicFieldResponse.message,
            });
            return HttpStatusCodes.NOT_FOUND;
        }

        if (primaryObjectResponse instanceof Error) {
            // The site object is gone but its dynamic field survived — we are
            // serving a deleted site's orphaned resource (SEW-1037).
            logger.warn("Site object missing but resource exists — serving orphaned resource", {
                objectId,
                dynamicFieldId,
            });
        }

        return this.extractResource(dynamicFieldResponse, dynamicFieldId);
    }

    /**
     * Fetches the data of a parentObject and its' dynamicFieldObject.
     * @param objectId: The objectId of the parentObject (e.g. site::Site).
     * @param dynamicFieldId: The Id of the dynamicFieldObject (e.g. site::Resource).
     * @returns A tuple [parentObject, dynamicFieldObject]; each entry is an `Error` if not found.
     */
    private async fetchObjectPairData(
        objectId: string,
        dynamicFieldId: string,
    ): Promise<SuiClientTypes.GetObjectsResponse<{ content: true; display: true }>["objects"]> {
        logger.info("Fetching Display object and Dynamic Field object", {
            objectIdForDisplay: objectId,
            dynamicFieldId,
        });
        // multiGetObjects returns the objects *always* in the order they were requested.
        return this.rpcSelector.multiGetObjects([objectId, dynamicFieldId], {
            content: true,
            display: true,
        });
    }

    /**
     * Extracts the resource data from the dynamicFieldObject.
     * @param dynamicFieldResponse: contains the data of the dynamicFieldObject
     * @param dynamicFieldId: The Id of the dynamicFieldObject (e.g. site::Resource).
     * @returns A VersionedResource or an HttpStatusCode in case of an error.
     */
    private extractResource(
        dynamicFieldResponse: SuiClientTypes.Object<{ content: true; display: true }>,
        dynamicFieldId: string,
    ): VersionedResource | HttpStatusCodes {
        logger.info("Extracting resource data from the dynamic field object", {
            dynamicFieldId,
        });
        const siteResource = this.getResourceFields(dynamicFieldResponse);
        if (!siteResource || !siteResource.blob_id) {
            logger.error("No site resource found inside the dynamicFieldResponse:", {
                dynamicFieldId,
                error: dynamicFieldResponse,
            });
            return HttpStatusCodes.NOT_FOUND;
        }

        return {
            ...siteResource,
            version: dynamicFieldResponse.version,
            objectId: dynamicFieldId,
        } as VersionedResource;
    }

    /**
     * Checks for loop detection and too many redirects.
     * @param objectId
     * @param seenResources
     * @param depth
     * @returns
     */
    private checkRedirectLimits(
        objectId: string,
        seenResources: Set<string>,
        depth: number,
    ): HttpStatusCodes | null {
        if (seenResources.has(objectId)) return HttpStatusCodes.LOOP_DETECTED;
        if (depth >= MAX_REDIRECT_DEPTH) return HttpStatusCodes.TOO_MANY_REDIRECTS;
        return null;
    }

    /**
     * Parses the resource information from the Sui object data response.
     */
    private getResourceFields(
        object: SuiClientTypes.Object<{ content: true; display: true }>,
    ): Resource | null {
        // Deserialize the BCS-encoded Move struct content of the dynamic field.
        if (object.content) {
            const df = DynamicFieldStruct(ResourcePathStruct, ResourceStruct).parse(object.content);
            return df.value;
        }
        return null;
    }
}
