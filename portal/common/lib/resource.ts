// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { HttpStatusCodes } from "./http/http_status_codes";
import { SuiClient, SuiObjectData, SuiObjectResponse } from "@mysten/sui/client";
import { Resource, VersionedResource } from "./types";
import { MAX_REDIRECT_DEPTH, RESOURCE_PATH_MOVE_TYPE } from "./constants";
import { checkRedirect } from "./redirects";
import { fromBase64 } from "@mysten/bcs";
import { ResourcePathStruct, DynamicFieldStruct, ResourceStruct } from "./bcs_data_parsing";
import { deriveDynamicFieldID } from "@mysten/sui/utils";
import { bcs } from "@mysten/bcs";

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
export async function fetchResource(
    client: SuiClient,
    objectId: string,
    path: string,
    seenResources: Set<string>,
    depth: number = 0,
): Promise<VersionedResource | HttpStatusCodes> {
    if (seenResources.has(objectId)) {
        return HttpStatusCodes.LOOP_DETECTED;
    }
    if (depth >= MAX_REDIRECT_DEPTH) {
        return HttpStatusCodes.TOO_MANY_REDIRECTS;
    }

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
    ] = await fetchObjectPairData(client, objectId, dynamicFieldId);

    seenResources.add(objectId);

    const redirectId = checkRedirect(primaryObjectResponse);
    if (redirectId) {
        return fetchResource(client, redirectId, path, seenResources, depth + 1);
    }

    // If no page data found.
    if (!dynamicFieldResponse.data) {
        console.log("No page data found");
        return HttpStatusCodes.NOT_FOUND;
    }
    const siteResource = getResourceFields(dynamicFieldResponse.data);
    if (!siteResource || !siteResource.blob_id) {
        return HttpStatusCodes.NOT_FOUND;
    }
    return {
        ...siteResource,
        version: dynamicFieldResponse.data.version,
        objectId: dynamicFieldId,
    } as VersionedResource;
}

/**
* Fetches the data of a parentObject and its' dynamicFieldObject.
* @param client: A SuiClient to interact with the Sui network.
* @param objectId: The objectId of the parentObject (e.g. site::Site).
* @param dynamicFieldId: The Id of the dynamicFieldObject (e.g. site::Resource).
* @returns A tuple of SuiObjectResponse[] or an HttpStatusCode in case of an error.
*/
async function fetchObjectPairData(
    client: SuiClient,
    objectId: string,
    dynamicFieldId: string
): Promise<SuiObjectResponse[]> {
    // MultiGetObjects returns the objects *always* in the order they were requested.
    const pageData = await client.multiGetObjects(
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
 * Parses the resource information from the Sui object data response.
 */
function getResourceFields(data: SuiObjectData): Resource | null {
    // Deserialize the bcs encoded struct
    if (data.bcs && data.bcs.dataType === "moveObject") {
        const df = DynamicFieldStruct(ResourcePathStruct, ResourceStruct).parse(
            fromBase64(data.bcs.bcsBytes),
        );
        return df.value;
    }
    return null;
}
