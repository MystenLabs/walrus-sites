// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import React from "react";
import NFTActionButtons from "./NFTActionButtons";
import { useNavigate } from "react-router-dom";
import { NUMBER_OF_WINNERS, SHOW_WINNERS } from "../config/globalVariables";

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
  position,
  imageSrc,
  title,
  creatorName,
  votes,
  onVoteSuccess,
}) => {
  const navigate = useNavigate();
  const positionBadgeImage =
    SHOW_WINNERS && position >= 3 && position <= NUMBER_OF_WINNERS
      ? "WinnerBanner.png"
      : `No${position}.png`;

  const badgeStyles =
    SHOW_WINNERS && position >= 3 && position <= NUMBER_OF_WINNERS
      ? "absolute top-0 left-0 w-auto h-12 mt-1 -ml-1"
      : "absolute top-0 left-0 w-14 h-12 mt-1 -ml-1";

  const openMemeDetailsPage = () => {
    navigate(`/meme-details/${nftId}`);
  };

  return (
    <div className="relative flex flex-col items-center bg-primary_dark rounded-lg min-h-full  p-2 ">
      <div
        className="relative  hover:cursor-pointer"
        onClick={openMemeDetailsPage}
      >
        <div className="custom_lg:w-[250px] custom_lg:h-[250px] w-full h-full ">
          <img
            src={imageSrc}
            alt={title}
            className="w-full h-full object-cover rounded-lg"
          />
        </div>

        <img src={positionBadgeImage} className={badgeStyles} />
      </div>

      <div className="custom_lg:w-[250px] w-full   mt-3 text-left">
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
