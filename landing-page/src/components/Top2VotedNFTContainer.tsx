// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import React from "react";
import NFTActionButtons from "./NFTActionButtons";
import { useNavigate } from "react-router-dom";
import { SHOW_WINNERS } from "../config/globalVariables";

interface TopVotedNFTProps {
  nftId: string;
  position: number;
  imageSrc: string;
  title: string;
  creatorName: string;
  votes: number;
  onVoteSuccess: () => void;
}

const TopVotedNFT: React.FC<TopVotedNFTProps> = ({
  nftId,
  position,
  imageSrc,
  title,
  creatorName,
  votes,
  onVoteSuccess,
}) => {
  const navigate = useNavigate();
  const gradientBorder =
    position === 1
      ? "bg-gradient-to-t from-[#C684F6] to-[#C684F600]"
      : "bg-gradient-to-t from-[#98EFE4] to-[#98EFE400]";
  const positionBadgeImage = SHOW_WINNERS
    ? "WinnerBanner.png"
    : position === 1
    ? "No1.png"
    : "No2.png";
  const badgeStyles = SHOW_WINNERS
    ? "absolute top-0 left-0 w-auto h-12 -mt-4 -ml-6"
    : "absolute top-0 left-0 w-18 h-16 -mt-4 -ml-6";

  const openMemeDetailsPage = () => {
    navigate(`/meme-details/${nftId}`);
  };

  return (
    <div className={`relative p-0.5 rounded-lg max-w-full ${gradientBorder} `}>
      <div className="bg-primary_dark px-4 pb-4 rounded-lg">
        <div
          className="relative flex flex-col lg:flex-row items-start rounded-lg px-4 pt-4 pb-2 bg-primary_dark max-w-full min-h-full xs:min-w-[500px] hover:cursor-pointer"
          onClick={openMemeDetailsPage}
        >
          <div className="relative lg:mr-4 mr-0">
            <div className="lg:w-[280px] lg:h-[280px]  w-full h-auto   p-2">
              <img
                src={imageSrc}
                alt={title}
                className="w-full h-full object-cover rounded-lg"
              />
            </div>

            <img src={positionBadgeImage} className={badgeStyles} />
          </div>

          <div className="flex-1 relative mt-4 md:mt-0">
            <h3 className="custom_lg:text-xl lg:text-lg text-base font-bold text-white custom_lg:max-w-[200px] max-w-[100%] pt-4 pb-1 px-2">
              {title}
            </h3>

            <div className="flex items-center space-x-2 pt-1 px-2">
              <div>
                <p className="text-xs text-gray-500">Created by</p>
                <p className="text-white text-xs">
                  {creatorName && creatorName.length > 20
                    ? `${creatorName.slice(0, 6)}...${creatorName.slice(-4)}`
                    : creatorName || "anonymous"}
                </p>
              </div>
            </div>
          </div>
        </div>
        <div className="px-4 ">
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

export default TopVotedNFT;
