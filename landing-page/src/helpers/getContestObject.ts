// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { SuiClient, SuiMoveObject } from "@mysten/sui/client";

export type ContestOnChain = {
  id: {
    id: string;
  };
  counter: number;
  nonce_bytes: number[];
  minted_nfts_vec: string[];
  end_time: number;
};

interface GetContestObjectProps {
  contestId: string;
  suiClient: SuiClient;
}
export const getContestObject = async ({
  suiClient,
  contestId,
}: GetContestObjectProps): Promise<ContestOnChain> => {
  const res = await suiClient.getObject({
    id: contestId,
    options: { showContent: true },
  });
  const contestObject = res?.data?.content as SuiMoveObject;
  const { fields } = contestObject;
  return fields as unknown as ContestOnChain;
};
