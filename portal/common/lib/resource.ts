// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { HttpStatusCodes } from "./http/http_status_codes";
import { SuiObjectData, SuiObjectResponse } from "@mysten/sui/client";
import { Resource, VersionedResource } from "./types";
import { MAX_REDIRECT_DEPTH, RESOURCE_PATH_MOVE_TYPE } from "./constants";
import { checkRedirect } from "./redirects";
import { fromBase64 } from "@mysten/bcs";
import { ResourcePathStruct, DynamicFieldStruct, ResourceStruct } from "./bcs_data_parsing";
import { deriveDynamicFieldID } from "@mysten/sui/utils";
import { bcs } from "@mysten/bcs";
import { RPCSelector } from "./rpc_selector";
import logger from "./logger";

export class ResourceFetcher {
    constructor(private rpcSelector: RPCSelector) {}

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
        const error = this.checkForErrors(objectId, seenResources, depth);
        if (error) return error;

        // The dynamic field object ID can be derived, without
        // making a request to the network.
        const dynamicFieldId = deriveDynamicFieldID(
            objectId,
            RESOURCE_PATH_MOVE_TYPE,
            bcs.string().serialize(path).toBytes(),
        );

        const [
            primaryObjectResponse,
            dynamicFieldResponse
        ] = await this.fetchObjectPairData(objectId, dynamicFieldId);

        seenResources.add(objectId);

        const redirectId = checkRedirect(primaryObjectResponse);
        if (redirectId) {
            return this.fetchResource(redirectId, path, seenResources, depth + 1);
        }

        return this.extractResource(dynamicFieldResponse, dynamicFieldId);
    }

    /**
    * Fetches the data of a parentObject and its' dynamicFieldObject.
    * @param client: A SuiClient to interact with the Sui network.
    * @param objectId: The objectId of the parentObject (e.g. site::Site).
    * @param dynamicFieldId: The Id of the dynamicFieldObject (e.g. site::Resource).
    * @returns A tuple of SuiObjectResponse[] or an HttpStatusCode in case of an error.
    */
    private async fetchObjectPairData(
        objectId: string,
        dynamicFieldId: string
    ): Promise<SuiObjectResponse[]> {
        // MultiGetObjects returns the objects *always* in the order they were requested.
        const pageData = await this.rpcSelector.multiGetObjects(
            {
                ids: [
                    objectId,
                    dynamicFieldId
                ],
                options: { showBcs: true, showDisplay: true }
            },
        );
        // MultiGetObjects returns the objects *always* in the order they were requested.
        const primaryObjectResponse: SuiObjectResponse = pageData[0];
        const dynamicFieldResponse: SuiObjectResponse = pageData[1];

        return [primaryObjectResponse, dynamicFieldResponse]
    }

    /**
    * Extracts the resource data from the dynamicFieldObject.
    * @param dynamicFieldResponse: contains the data of the dynamicFieldObject
    * @param dynamicFieldId: The Id of the dynamicFieldObject (e.g. site::Resource).
    * @returns A VersionedResource or an HttpStatusCode in case of an error.
    */
    private extractResource(
        dynamicFieldResponse: SuiObjectResponse,
        dynamicFieldId: string): VersionedResource | HttpStatusCodes
    {
        if (!dynamicFieldResponse.data) {
            logger.warn({
                message: "No page data found for dynamic field id",
                dynamicFieldId: dynamicFieldId
            });
            return HttpStatusCodes.NOT_FOUND;
        }

        const siteResource = this.getResourceFields(dynamicFieldResponse.data);
        if (!siteResource || !siteResource.blob_id) {
            logger.error({
                message: "No site resource found inside the dynamicFieldResponse:",
                error: dynamicFieldResponse
            });
            return HttpStatusCodes.NOT_FOUND;
        }

        return {
            ...siteResource,
            version: dynamicFieldResponse.data.version,
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
    private checkForErrors(
        objectId: string,
        seenResources: Set<string>, depth: number
    ): HttpStatusCodes | null {
        if (seenResources.has(objectId)) return HttpStatusCodes.LOOP_DETECTED;
        if (depth >= MAX_REDIRECT_DEPTH) return HttpStatusCodes.TOO_MANY_REDIRECTS;
        return null;
    }

    /**
     * Parses the resource information from the Sui object data response.
     */
    private getResourceFields(data: SuiObjectData): Resource | null {
        // Deserialize the bcs encoded struct
        if (data.bcs && data.bcs.dataType === "moveObject") {
            const df = DynamicFieldStruct(ResourcePathStruct, ResourceStruct).parse(
                fromBase64(data.bcs.bcsBytes),
            );
            return df.value;
        }
        return null;
    }
}
