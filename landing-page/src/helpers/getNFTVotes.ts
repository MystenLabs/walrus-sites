// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { SuiClient } from "@mysten/sui/client";
import { PACKAGE_ID } from "../config/globalVariables";

interface GetVoteNftIdProps {
  memeNftId: string;
  suiClient: SuiClient;
}
interface GetVotesProps {
  cursor: any | null;
  memeNftId: string;
  suiClient: SuiClient;
}

async function getVotes({
  suiClient,
  cursor,
  memeNftId,
}: GetVotesProps): Promise<any> {
  return await suiClient.getOwnedObjects({
    owner: memeNftId,
    cursor,
    filter: {
      StructType: `${PACKAGE_ID}::vote::Vote`,
    },
    options: {
      showContent: true,
      showType: true,
    },
  });
}

export const getNFTVotes = async ({
  suiClient,
  memeNftId,
}: GetVoteNftIdProps): Promise<number> => {
  let votesCount = 0;
  let res = await getVotes({ suiClient, cursor: null, memeNftId });

  votesCount = res.data.length;

  while (res.hasNextPage) {
    res = await getVotes({ suiClient, cursor: res.nextCursor, memeNftId });
    votesCount += res.data.length;
  }

  console.log("votesCount = ", votesCount);
  return votesCount;
};
