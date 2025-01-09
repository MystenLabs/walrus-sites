// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import React, { useEffect, useState } from "react";
import { useNavigate } from "react-router-dom";
import { getMemeNftObject } from "../helpers/getMemeNFTObject";
import { toast } from "react-toastify";
import { useSuiClient } from "@mysten/dapp-kit";
import { SuiClient } from "@mysten/sui/client";
import { BACKEND_URL } from "../config/globalVariables";

interface UserVoteContainerProps {
  nftId: string;
}

const UserVoteContainer: React.FC<UserVoteContainerProps> = ({ nftId }) => {
  const navigate = useNavigate();
  const handleClick = () => {
    navigate(`/meme-details/${nftId}`);
  };

  const [memeTitle, setMemeTitle] = useState<string>("Loading...");
  const [blobId, setBlobId] = useState<string>("");
  const [votes, setVotes] = useState<number>(0);
  const suiClient = useSuiClient();

  useEffect(() => {
    const fetchData = async () => {
      if (!nftId) return;

      try {
        const nftObject = await getMemeNftObject(
          nftId,
          suiClient as unknown as SuiClient
        );

        if (nftObject) {
          setMemeTitle(nftObject.title || "Untitled Meme");
          setBlobId(nftObject.blob_id);

          const response = await fetch(
            `${BACKEND_URL}/api/contestUser/nftVotes`,
            {
              method: "POST",
              headers: { "Content-Type": "application/json" },
              body: JSON.stringify({ nftIds: [nftObject.id.id] }),
            }
          );

          const votesData = await response.json();

          if (Array.isArray(votesData) && votesData.length > 0) {
            const voteInfo = votesData.find(
              (vote: { id: string }) => vote.id === nftObject.id.id
            );
            setVotes(voteInfo?.votes || 0);
          } else {
            setVotes(0);
          }
        }
      } catch (error) {
        console.error("Error fetching NFT details:", error);
        toast.error("Failed to load NFT details. Please try again later.");
      }
    };

    fetchData();
  }, [nftId, suiClient]);

  return (
    <div
      className="bg-[#161A30] text-black font-bold py-3 px-3 rounded-lg space-x-2 max-w-[400px] w-full flex relative hover:cursor-pointer"
      onClick={handleClick}
    >
      <img
        src={`https://aggregator.walrus-testnet.walrus.space/v1/${blobId}`}
        alt="Submission"
        className="w-20 h-20 rounded-lg"
      />
      <div className="pl-4 pt-1">
        <p className="text-xl text-white ">{memeTitle || "Untitled Meme"}</p>
      </div>
      <div
        className={
          "absolute bottom-1 right-1 flex items-center justify-center rounded-md font-semibold border text-white border-primary_teal bg-[#97F0E54D]"
        }
        style={{
          padding: "8px",
          margin: "8px",
        }}
      >
        <img
          src="/Like_Button.png"
          alt="Like"
          style={{
            width: `16px`,
            height: `16px`,
          }}
        />
        <span className="ml-2 text-xs">{votes}</span>
      </div>
    </div>
  );
};

export default UserVoteContainer;
