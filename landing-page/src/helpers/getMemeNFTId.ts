// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { SuiClient } from "@mysten/sui/client";
import { PACKAGE_ID } from "../config/globalVariables";

interface GetMemeNftIdProps {
  suiClient: SuiClient;
  address: string;
}

//returns array of obj ids
export const getMemeNftId = async ({
  suiClient,
  address,
}: GetMemeNftIdProps): Promise<string[] | undefined> => {
  return suiClient
    .getOwnedObjects({
      owner: address,
      filter: {
        StructType: `${PACKAGE_ID}::meme::MemeNFT`,
      },
    })
    .then(async (res) => {
      const objects = res?.data;
      const ret: string[] = [];
      objects.forEach((obj) => {
        if (!obj.data) {
          throw new Error(`Object data is undefined: ${obj.error}`);
        }
        ret.push(obj.data.objectId);
      });
      if (ret.length >= 1) return ret;
      return undefined;
    })
    .catch((err) => {
      console.log(err);
      return undefined;
    });
};
