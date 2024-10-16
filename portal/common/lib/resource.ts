// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { HttpStatusCodes } from "./http/http_status_codes";
import { SuiClient, SuiObjectData } from "@mysten/sui/client";
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

    const redirectPromise = checkRedirect(client, objectId);
    seenResources.add(objectId);

    const dynamicFieldId = deriveDynamicFieldID(
        objectId,
        RESOURCE_PATH_MOVE_TYPE,
        bcs.string().serialize(path).toBytes(),
    );
    console.log("Derived dynamic field objectID: ", dynamicFieldId);

    // Fetch page data.
    const pageData = await client.getObject({
        id: dynamicFieldId,
        options: { showBcs: true },
    });

    // If no page data found.
    if (!pageData || !pageData.data) {
        const redirectId = await redirectPromise;
        if (redirectId) {
            return fetchResource(client, redirectId, path, seenResources, depth + 1);
        }
        console.log("No page data found");
        return HttpStatusCodes.NOT_FOUND;
    }

    const siteResource = getResourceFields(pageData.data);
    if (!siteResource || !siteResource.blob_id) {
        return HttpStatusCodes.NOT_FOUND;
    }

    return {
        ...siteResource,
        version: pageData.data?.version,
        objectId: dynamicFieldId,
    } as VersionedResource;
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
