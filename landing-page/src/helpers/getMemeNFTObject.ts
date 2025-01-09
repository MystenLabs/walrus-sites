// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { SuiClient, SuiMoveObject } from "@mysten/sui/client";

interface MemeNftOnChain {
  id: {
    id: string;
  };
  title: string;
  creator: string;
  blob_id: string;
  contest_id: string;
}

export const getMemeNftObject = async (
  nftId: string,
  suiClient: SuiClient
): Promise<MemeNftOnChain | null> => {
  try {
    const res = await suiClient.getObject({
      id: nftId,
      options: { showContent: true },
    });

    if (res.data && res.data.content) {
      const contestObject = res.data.content as SuiMoveObject;
      const fields = contestObject.fields as unknown as MemeNftOnChain;

      if (
        fields &&
        typeof fields.id?.id === "string" &&
        typeof fields.title === "string" &&
        typeof fields.creator === "string" &&
        typeof fields.blob_id === "string"
      ) {
        return fields; // Return the structured MemeNftOnChain object
      } else {
        console.error("Invalid NFT fields structure");
      }
    }
  } catch (error) {
    console.error(`Error fetching NFT details for ID ${nftId}:`, error);
  }

  return null; // Return null if fetching fails
};
