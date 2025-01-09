// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import React from "react";
import NFTActionButtons from "./NFTActionButtons";
import { useNavigate } from "react-router-dom";

interface Top10VotedNFTProps {
  nftId: string;
  position: number;
  imageSrc: string;
  title: string;
  creatorName: string;
  votes: number;
  onVoteSuccess: () => void;
}

const Top10VotedNFT: React.FC<Top10VotedNFTProps> = ({
  nftId,
  imageSrc,
  title,
  creatorName,
  votes,
  onVoteSuccess,
}) => {
  const navigate = useNavigate();
  const openMemeDetailsPage = () => {
    navigate(`/meme-details/${nftId}`);
  };
  return (
    <div className="relative flex flex-col items-center bg-primary_dark rounded-lg h-[350px]">
      <div
        className="relative hover:cursor-pointer"
        onClick={openMemeDetailsPage}
      >
        <div className="custom_lg:w-[200px] custom_lg:h-[200px] w-full h-full">
          <img
            src={imageSrc}
            alt={title}
            className="w-full h-full object-cover rounded-lg"
          />
        </div>
      </div>

      <div className="custom_lg:w-[200px] custom_lg:h-[200px] w-full h-full mt-3 text-left">
        <h3 className="text-base font-semibold text-white">{title}</h3>
        <div className="flex items-center mt-1 space-x-1">
          <span className="text-sm text-gray-400">
            {creatorName && creatorName.length > 20
              ? `${creatorName.slice(0, 6)}...${creatorName.slice(-4)}`
              : creatorName || "anonymous"}
          </span>
        </div>
        <div className="pt-4">
          <NFTActionButtons
            votes={votes}
            nftId={nftId}
            onVoteSuccess={onVoteSuccess}
          />
        </div>
      </div>
    </div>
  );
};

export default Top10VotedNFT;
