// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import React, { useEffect, useState } from "react";
import UnifiedShareButton from "./UnifiedShareButton";
import ReportNFTPopup from "./ReportNFTPopup";
import { NETWORK_NAME } from "../config/globalVariables";
import { getMemeNftObject } from "../helpers/getMemeNFTObject";
import { useSuiClient } from "@mysten/dapp-kit";
import { SuiClient } from "@mysten/sui/client";

interface NFTActionPopupProps {
  onClose: () => void;
  nftId: string;
}

const NFTActionPopup: React.FC<NFTActionPopupProps> = ({ onClose, nftId }) => {
  const [isReportPopupOpen, setIsReportPopupOpen] = useState(false);
  const [blobId, setBlobId] = useState<string>("");
  const suiClient = useSuiClient();

  useEffect(() => {
    const fetchNFTObject = async () => {
      try {
        const nftObject = await getMemeNftObject(
          nftId,
          suiClient as unknown as SuiClient
        );
        if (nftObject && nftObject.blob_id) {
          setBlobId(nftObject.blob_id);
        } else {
          console.error("NFT object is missing blob_id.");
        }
      } catch (error) {
        console.error("Error fetching NFT object:", error);
      }
    };

    fetchNFTObject();
  }, [nftId, suiClient]);

  const handleOpenReportPopup = () => {
    setIsReportPopupOpen(true);
  };

  const handleCloseReportPopup = () => {
    setIsReportPopupOpen(false);
  };

  return (
    <>
      {/* Main Popup Content */}
      <div className="bg-[#1E233B] text-white rounded-lg shadow-lg p-3 w-64 relative">
        <div className="relative mb-2">
          <div className="flex justify-start">
            <UnifiedShareButton nftId={nftId} size={32} />
          </div>

          <button
            onClick={onClose}
            className="absolute top-1 right-1 text-gray-400 hover:text-gray-200 focus:outline-none z-50"
            aria-label="Close"
          >
            ✕
          </button>
        </div>

        <div className="border-t border-white border-opacity-50 mb-2"></div>

        <a
          className="flex-1 flex items-center justify-between bg-[#252B4A] py-2.5 px-4 rounded-lg text-white text-xs mb-2"
          href={`https://walruscan.com/testnet/blob/${blobId}`}
          target="_blank"
          rel="noopener noreferrer"
        >
          <img src="/Sui_Explorer.png" alt="Left Icon" className="w-4 h-4" />
          View on Walrus Explorer
          <img src="/Arrow_Up.png" alt="Right Icon" className="w-4 h-4" />
        </a>
        <a
          className="flex-1 flex items-center justify-between bg-[#252B4A] mb-2 py-2.5 px-4 rounded-lg text-white text-xs"
          href={`https://suiscan.xyz/${NETWORK_NAME}/object/${nftId}`}
          target="_blank"
          rel="noopener noreferrer"
        >
          <img src="/Sui_Scan.png" alt="Left Icon" className="w-4 h-4" />
          View in SuiScan
          <img src="/Arrow_Up.png" alt="Right Icon" className="w-4 h-4" />
        </a>

        <div className="border-t border-white border-opacity-50 mb-2"></div>

        <div className="text-left">
          <button
            onClick={handleOpenReportPopup}
            className="text-[#FF5C5C] hover:underline py-2.5 px-4 rounded-lg text-xs"
          >
            <img
              src="/Red_Flag.png"
              alt="Flag"
              className="inline-block w-4 h-4 mr-3"
            />
            Flag this Meme
          </button>
        </div>
      </div>

      {isReportPopupOpen && (
        <ReportNFTPopup memeNftId={nftId} onClose={handleCloseReportPopup} />
      )}
    </>
  );
};

export default NFTActionPopup;
