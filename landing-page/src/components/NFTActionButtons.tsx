// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import React, { useState, useEffect, useRef } from "react";
import NFTActionPopup from "./NFTActionPopup";
import { CONTEST_ID, VOTE_IS_OPEN } from "../config/globalVariables";
import { SuiClient } from "@mysten/sui/client";
import {
  useCurrentAccount,
  useSignTransaction,
  useSuiClient,
} from "@mysten/dapp-kit";
import { useGoogleReCaptcha } from "react-google-recaptcha-v3";
import {
  executeSponsorshipTransaction,
  fetchTransactionDetails,
  getReCaptchaToken,
  sendSponsorRequest,
  signTransaction,
  waitForTransactionCompletion,
} from "../helpers/voteNFT";
import { toast } from "react-toastify";

interface NFTActionButtonsProps {
  nftId: string;
  votes: number;
  size?: number;
  hideReportButton?: boolean;
  onVoteSuccess?: () => void;
}

const NFTActionButtons: React.FC<NFTActionButtonsProps> = ({
  nftId,
  votes,
  size = 40, // Default size
  hideReportButton,
  onVoteSuccess,
}) => {
  const [isLiked, setIsLiked] = useState(false);
  const [isPopupOpen, setIsPopupOpen] = useState(false); // Track popup state
  const popupRef = useRef<HTMLDivElement>(null); // Reference for the popup

  const iconSize = size * 0.4; // Icon size relative to button size
  const rectButtonHeight = size; // Rectangle button height matches the size
  const rectButtonWidth = size * 1.7; // Rectangle button is wider than square buttons
  const { executeRecaptcha } = useGoogleReCaptcha();
  const handleReCaptchaVerify = async () => {
    if (!executeRecaptcha) return null;
    return await executeRecaptcha("vote_nft");
  };
  const account = useCurrentAccount();
  const suiClient = useSuiClient();
  const { mutateAsync: signTransactionBlock } = useSignTransaction();

  const handleLikeClick = async () => {
    if (account?.address === undefined) {
      toast.error("Please connect your wallet to vote for a meme.");
      return;
    }
    setIsLiked(true);

    try {
      const token = await getReCaptchaToken(handleReCaptchaVerify);

      const { txBytes: sponsoredBytes, txDigest } = await sendSponsorRequest(
        CONTEST_ID,
        nftId,
        account?.address || "",
        token
      );

      const signature = await signTransaction(
        signTransactionBlock,
        sponsoredBytes
      );

      const { digest: executedDigest } = await executeSponsorshipTransaction(
        txDigest,
        signature
      );

      await waitForTransactionCompletion(
        suiClient as unknown as SuiClient,
        executedDigest
      );

      await fetchTransactionDetails(
        suiClient as unknown as SuiClient,
        executedDigest
      );

      toast.success("Vote submitted successfully!");
      if (onVoteSuccess) {
        onVoteSuccess(); // Trigger the refresh on HomePage
      }
    } catch (error) {
      console.error("Error during voting process:", error);
      toast.error("Error submitting your vote. Please try again.");
    } finally {
      // Revert the background state after 5 seconds
      setTimeout(() => {
        setIsLiked(false);
      }, 5000);
    }
  };

  const togglePopup = () => {
    setIsPopupOpen((prev) => !prev); // Toggle popup state
  };

  const handleClosePopup = () => {
    setIsPopupOpen(false); // Close popup
  };

  // Close popup when pressing the Escape key
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        handleClosePopup();
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => {
      window.removeEventListener("keydown", handleKeyDown);
    };
  }, []);

  // Close popup when clicking outside
  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      if (
        popupRef.current &&
        !popupRef.current.contains(e.target as Node) &&
        (e.target as HTMLElement).id !== "dots-button"
      ) {
        handleClosePopup();
      }
    };
    window.addEventListener("mousedown", handleClickOutside);
    return () => {
      window.removeEventListener("mousedown", handleClickOutside);
    };
  }, []);

  return (
    <div
      className={`relative flex ${
        VOTE_IS_OPEN ? "justify-between" : "justify-end"
      } items-center w-full`}
    >
      {VOTE_IS_OPEN && (
        <button
          onClick={handleLikeClick}
          className={`flex items-center justify-center rounded-md font-semibold border text-white ${
            isLiked
              ? "border-primary_teal bg-[#97F0E54D]"
              : "border-primary_dark bg-[#222741] "
          }`}
          style={{
            height: `${rectButtonHeight}px`,
            width: `${rectButtonWidth}px`,
          }}
        >
          <img
            src="/Like_Button.png"
            alt="Like"
            style={{
              width: `${iconSize}px`,
              height: `${iconSize}px`,
            }}
          />
          <span className="ml-2 text-xs">{votes}</span>
        </button>
      )}

      {!hideReportButton && (
        <button
          id="dots-button" // Add ID to prevent conflicts with outside click logic
          onClick={togglePopup}
          className={`flex items-center justify-center rounded-md font-semibold border text-white ${
            isPopupOpen
              ? "border-primary_teal bg-primary_teal"
              : "border-primary_dark bg-[#222741]"
          }`}
          style={{
            height: `${rectButtonHeight}px`,
            width: `${rectButtonHeight}px`,
          }}
        >
          <img
            src={isPopupOpen ? "/dots_dark.png" : "/dots_white.png"}
            alt="Actions"
            style={{
              width: `${iconSize}px`,
              height: `${iconSize}px`,
            }}
          />
        </button>
      )}

      {isPopupOpen && (
        <div
          ref={popupRef}
          className="absolute top-full mt-2 right-10 w-[200px] z-50"
        >
          <NFTActionPopup nftId={nftId} onClose={handleClosePopup} />
        </div>
      )}
    </div>
  );
};

export default NFTActionButtons;
