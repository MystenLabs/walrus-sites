// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import React, { useEffect, useState } from "react";
import NFTActionButtons from "./NFTActionButtons";
import { toast } from "react-toastify";
import "react-toastify/dist/ReactToastify.css";
import { SuiClient } from "@mysten/sui/client";
import UnifiedShareButton from "./UnifiedShareButton";
import {
  BACKEND_URL,
  CONTEST_ID,
  NETWORK_NAME,
} from "../config/globalVariables";
import { useNavigate } from "react-router-dom";
import { getMemeNftObject } from "../helpers/getMemeNFTObject";
import ReportNFTPopup from "./ReportNFTPopup";
import {
  useCurrentAccount,
  useSignTransaction,
  useSuiClient,
} from "@mysten/dapp-kit";
import { useGoogleReCaptcha } from "react-google-recaptcha-v3";
import { executeSponsorship } from "../helpers/submitNFT";
import { sponsorDeleteTransaction } from "../helpers/deleteNFT";

interface PopupProps {
  isOpen: boolean;
  onClose: () => void;
  nftId: string | null;
  onVoteSuccess: () => void;
}

const MemeDetails: React.FC<PopupProps> = ({
  isOpen,
  onClose,
  nftId,
  onVoteSuccess,
}) => {
  const navigate = useNavigate();
  const [memeTitle, setMemeTitle] = useState<string>("Loading...");
  const [imageSrc, setImageSrc] = useState<string>("");
  const [blobId, setBlobId] = useState<string>("");
  const [creator, setCreator] = useState<string>("");
  const [votes, setVotes] = useState<number>(0);
  const [objectId, setObjectId] = useState<string>("");
  const [isReportPopupOpen, setIsReportPopupOpen] = useState(false);
  const account = useCurrentAccount();
  const [deletingNFT, setDeletingNFT] = useState(false);
  const [isDeleteConfirmation, setIsDeleteConfirmation] = useState(false);
  const { executeRecaptcha } = useGoogleReCaptcha();
  const { mutateAsync: signTransactionBlock } = useSignTransaction();
  const suiClient = useSuiClient();

  const handleReCaptchaVerify = async () => {
    if (!executeRecaptcha) return null;
    return await executeRecaptcha("delete_meme_nft");
  };

  const handleShowDeleteConfirmation = () => {
    setIsDeleteConfirmation(true);
  };

  const handleConfirmDeleteNFT = async () => {
    try {
      setDeletingNFT(true);
      let txDigest: string;
      const token = await handleReCaptchaVerify();
      const sponsorResponse = await sponsorDeleteTransaction(BACKEND_URL, {
        contestId: CONTEST_ID,
        creator: account?.address!,
        memeNftId: nftId!,
        token: token!,
        userAction: "delete_meme_nft",
      });

      const { signature } = await signTransactionBlock({
        transaction: sponsorResponse.txBytes,
      });

      if (!signature) {
        throw new Error("Signature generation failed.");
      }

      txDigest = await executeSponsorship(
        BACKEND_URL,
        sponsorResponse.txDigest,
        signature
      );

      await suiClient.waitForTransaction({
        digest: txDigest,
        timeout: 10_000,
      });

      setDeletingNFT(false);
      toast.success("NFT deleted successfully!");
      if (onVoteSuccess) {
        onVoteSuccess(); // Trigger the refresh on HomePage
      }
      navigate("/");
      setIsDeleteConfirmation(false);
    } catch (error) {
      console.error("Error deleting NFT:", error);
      toast.error("An unexpected error occurred while deleting the NFT.");
    }
  };

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
          setImageSrc(
            `https://aggregator.walrus-testnet.walrus.space/v1/${nftObject.blob_id}` ||
              "/NotFound.png"
          );
          setBlobId(nftObject.blob_id);
          setCreator(nftObject.creator || "anonymous");
          setObjectId(nftObject.id.id);

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
            setVotes(0); // Default to 0 if no votes are found
          }
        }
      } catch (error) {
        console.error("Error fetching NFT details:", error);
        toast.error("Failed to load NFT details. Please try again later.");
      }
    };

    fetchData();
  }, [nftId, onVoteSuccess]);

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        onClose();
        setIsDeleteConfirmation(false);
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => {
      window.removeEventListener("keydown", handleKeyDown);
    };
  }, [onClose]);

  const handleCopyToClipboard = (text: string) => {
    navigator.clipboard.writeText(text);
    toast.success("Copied to clipboard!");
  };

  const handleOpenReportPopup = () => {
    setIsReportPopupOpen(true);
  };

  const handleCloseReportPopup = () => {
    setIsReportPopupOpen(false);
  };

  if (!isOpen) return null;

  return (
    <>
      <div
        className="fixed inset-0 z-50 flex items-center justify-center bg-black bg-opacity-90 p-2"
        onClick={() => {
          onClose();
          setIsDeleteConfirmation(false);
        }} // Close on clicking the overlay
      >
        <div
          className="bg-[#13172A] rounded-lg shadow-lg p-6 max-w-md w-full relative text-white "
          onClick={(e) => e.stopPropagation()} // Prevent overlay click from closing the popup
        >
          <button
            onClick={onClose}
            className="absolute top-3 right-3 text-gray-500 hover:text-gray-700 focus:outline-none"
            aria-label="Close"
          >
            ✕
          </button>

          <h2 className="text-3xl font-ppNeueMontrealBold text-left pb-4">
            {!isDeleteConfirmation ? memeTitle : "Delete meme"}
          </h2>
          {isDeleteConfirmation && (
            <p className="text-gray-400 text-xs pb-4">
              This action will permanently destroy and remove this NFT from your
              wallet. This action can not be undone.
            </p>
          )}
          {!isDeleteConfirmation ? (
            <div className="flex">
              <img
                src={imageSrc}
                alt="Meme Preview"
                className="rounded-lg w-full h-auto"
              />
            </div>
          ) : (
            <div className="max-w-[400px] mx-auto bg-[#0D101E] rounded-lg p-6">
              <div className="w-[350px] h-[350px] mx-auto flex justify-center items-center mb-4 overflow-hidden">
                <img
                  src={imageSrc}
                  alt="Meme Preview"
                  className="rounded-lg w-full h-full object-cover"
                />
              </div>
              <div className="text-left">
                <h3 className="text-white text-lg mb-2">
                  {memeTitle || "Untitled Meme"}
                </h3>

                <p
                  className="text-gray-400 text-sm truncate hover:whitespace-normal"
                  title={account?.address || "anonymous"}
                >
                  @
                  {account?.address && account?.address.length > 20
                    ? `${account?.address.slice(
                        0,
                        6
                      )}...${account?.address.slice(-4)}`
                    : account?.address || "anonymous"}
                </p>
              </div>
            </div>
          )}

          {!isDeleteConfirmation && (
            <>
              <div className="flex justify-between items-center mt-6">
                <div className="flex flex-shrink-0">
                  <NFTActionButtons
                    nftId={nftId!}
                    votes={votes}
                    size={30}
                    hideReportButton={true}
                    onVoteSuccess={onVoteSuccess}
                  />
                </div>

                <div className="flex flex-shrink-0">
                  <UnifiedShareButton nftId={nftId!} size={30} />
                </div>
              </div>
              <div className="mt-4">
                <div className="flex justify-between items-center bg-[#222741] py-2 px-4 rounded-lg mb-0.5">
                  <span className="text-white text-sm">Walrus Blob ID</span>
                  <div className="flex items-center space-x-2">
                    <span className="text-gray-400 text-sm truncate max-w-[100px]">
                      {blobId
                        ? `${blobId.slice(0, 6)}...${blobId.slice(-4)}`
                        : ""}
                    </span>
                    <a
                      href={`https://walruscan.com/testnet/blob/${blobId}`}
                      target="_blank"
                      rel="noopener noreferrer"
                    >
                      <img
                        src="/Arrow_Up.png"
                        alt="Right Icon"
                        className="w-4 h-4"
                      />
                    </a>
                    <button onClick={() => handleCopyToClipboard(blobId)}>
                      <img
                        src="/Copy_Icon2.png"
                        alt="Copy"
                        className="w-3 h-3"
                      />
                    </button>
                  </div>
                </div>

                <div className="flex justify-between items-center bg-[#222741] py-2 px-4 rounded-lg">
                  <span className="text-white text-sm">Sui Object ID</span>
                  <div className="flex items-center space-x-2">
                    <span className="text-gray-400 text-sm truncate max-w-[100px]">
                      {objectId
                        ? `${objectId.slice(0, 6)}...${objectId.slice(-4)}`
                        : ""}
                    </span>
                    <a
                      href={`https://suiscan.xyz/${NETWORK_NAME}/object/${objectId}`}
                      target="_blank"
                      rel="noopener noreferrer"
                    >
                      <img
                        src="/Arrow_Up.png"
                        alt="Right Icon"
                        className="w-4 h-4"
                      />
                    </a>
                    <button onClick={() => handleCopyToClipboard(objectId)}>
                      <img
                        src="/Copy_Icon2.png"
                        alt="Copy"
                        className="w-3 h-3"
                      />
                    </button>
                  </div>
                </div>
              </div>{" "}
            </>
          )}
          {isDeleteConfirmation && (
            <div className="mt-6 flex flex-col">
              <div className="border border-red-500 rounded-lg p-2 flex  bg-[#1A1523] text-red-500 w-full">
                <img src="/Red_Check.png" alt="Warning" className="w-5 h-5" />
                <p className="text-white font-medium text-sm flex-grow text-center">
                  I understand this cannot be undone.
                </p>
              </div>

              <div className="flex justify-end space-x-4 mt-6">
                <button
                  onClick={() => setIsDeleteConfirmation(false)} // Cancel Confirmation
                  className="bg-gray-500 text-black text-base font-bold py-2 px-4 rounded-lg hover:opacity-90"
                >
                  CANCEL
                </button>
                <button
                  onClick={handleConfirmDeleteNFT} // Confirm Deletion
                  className="bg-[#F45858] text-black text-base font-bold py-2 px-4 rounded-lg hover:opacity-90"
                >
                  {deletingNFT ? "BURNING NFT..." : "DELETE"}
                </button>
              </div>
            </div>
          )}
          {!isDeleteConfirmation && (
            <div className="mt-6 flex justify-between items-center rounded-lg">
              <div className="flex items-center space-x-3">
                <div>
                  <span className="text-gray-400 text-xs">Created by</span>
                  <p className="text-white ">{`@${creator.slice(
                    0,
                    6
                  )}...${creator.slice(-4)}`}</p>
                </div>
              </div>

              {creator === account?.address && (
                // Show Delete Button
                <button
                  onClick={handleShowDeleteConfirmation} // Show Confirmation Screen
                  className="bg-[#F45858] text-black text-base font-bold py-2 px-4 rounded-lg hover:opacity-90"
                >
                  DELETE
                </button>
              )}
              {creator != account?.address && (
                <button
                  onClick={handleOpenReportPopup}
                  className="text-gray-400 text-sm flex items-center hover:underline"
                >
                  <img
                    src="/Report_Icon.png"
                    alt="Report"
                    className="w-auto h-3 mr-2 mt-6"
                  />
                </button>
              )}
            </div>
          )}
        </div>
      </div>

      {isReportPopupOpen && (
        <ReportNFTPopup memeNftId={nftId!} onClose={handleCloseReportPopup} />
      )}
    </>
  );
};

export default MemeDetails;
