// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import React, { useEffect, useState } from "react";
import FileUploader from "./FileUploader";
import { useSignTransaction, useSuiClient } from "@mysten/dapp-kit";
import { SuiClient } from "@mysten/sui/client";
import { useGoogleReCaptcha } from "react-google-recaptcha-v3";
import { toast } from "react-toastify";
import "react-toastify/dist/ReactToastify.css";
import { getMemeNftId } from "../helpers/getMemeNFTId";
import { executeSponsorship, sponsorTransaction } from "../helpers/submitNFT";
import UnifiedShareButton from "./UnifiedShareButton";
import {
  BACKEND_URL,
  CONTEST_ID,
  NETWORK_NAME,
} from "../config/globalVariables";

interface SubmitNFTProps {
  address: string | undefined;
  isOpen: boolean;
  onClose: () => void;
  onSubmitSuccess?: () => void;
}

const SubmitNFT: React.FC<SubmitNFTProps> = ({
  isOpen,
  onClose,
  address,
  onSubmitSuccess,
}) => {
  const [memeNftId, setMemeNftId] = useState<string[] | undefined>([
    "undefined",
  ]);
  const suiClient = useSuiClient();
  const { mutateAsync: signTransactionBlock } = useSignTransaction();
  const [uploadedFile, setUploadedFile] = useState<File | null>(null);
  const [memeTitle, setMemeTitle] = useState<string>("");
  const [isPublishEnabled, setIsPublishEnabled] = useState(false);
  const [blobId, setBlobId] = useState<string | null>(null);
  const [NFTId, setNFTId] = useState<string>("");
  const [isSubmitting, setIsSubmitting] = useState(false); // Loader state
  const [isSuccess, setIsSuccess] = useState(false); // Success state
  const { executeRecaptcha } = useGoogleReCaptcha();

  const handleReCaptchaVerify = async () => {
    if (!executeRecaptcha) return null;
    return await executeRecaptcha("mint_nft");
  };

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape" && isOpen) {
        handleClosePopup();
      }
    };
    window.addEventListener("keydown", handleKeyDown);

    return () => {
      window.removeEventListener("keydown", handleKeyDown);
    };
  }, [isOpen]);

  useEffect(() => {
    setIsPublishEnabled(!!uploadedFile && memeTitle.trim() !== "");
  }, [uploadedFile, memeTitle]);

  const handleFileUpload = (file: File, returnedBlobId: string) => {
    setUploadedFile(file);
    setBlobId(returnedBlobId); // Set blobId in state
  };

  const handleFileRemove = () => {
    setUploadedFile(null);
    setBlobId(null); // Clear the blobId on file removal
  };

  const handleClosePopup = () => {
    setUploadedFile(null);
    setMemeTitle("");
    setIsPublishEnabled(false);
    setIsSubmitting(false); // Reset submitting state
    setIsSuccess(false); // Reset success state
    onClose();
  };

  const handleSubmitNFT = async () => {
    setIsSubmitting(true); // Start loader
    const token = await handleReCaptchaVerify();
    if (!token) {
      toast.error("reCAPTCHA verification failed. Please try again.");
      setIsSubmitting(false); // Stop loader
      return;
    }

    try {
      let txDigest: string;
      let memeNftId: string[] | undefined;

      // Step 2: Sponsor transaction
      const sponsorResponse = await sponsorTransaction(BACKEND_URL, {
        contestId: CONTEST_ID,
        memeTitle: memeTitle!,
        address: address!,
        blobId: blobId!,
        token: token,
        userAction: "mint_nft",
      });

      const { signature } = await signTransactionBlock({
        transaction: sponsorResponse.txBytes,
        chain: `sui::${NETWORK_NAME}`,
      });

      if (!signature) {
        throw new Error("Signature generation failed.");
      }

      // Step 4: Execute the transaction
      txDigest = await executeSponsorship(
        BACKEND_URL,
        sponsorResponse.txDigest,
        signature
      );
      console.log("Execution Digest:", txDigest);

      // Step 5: Wait for transaction to complete
      await suiClient.waitForTransaction({
        digest: txDigest,
        timeout: 10_000,
      });

      // Step 6: Fetch MemeNFT IDs
      memeNftId = await getMemeNftId({
        suiClient: suiClient as unknown as SuiClient,
        address: address!,
      });

      if (!memeNftId || memeNftId.length === 0) {
        throw new Error("No MemeNFTs found for the user.");
      }

      console.log("Meme NFT objects for address:", address, "are:", memeNftId);
      setMemeNftId(memeNftId);

      setNFTId(memeNftId[0]);
      toast.success("NFT submitted successfully!");
      if (onSubmitSuccess) {
        onSubmitSuccess(); // Trigger the refresh on HomePage
      }
      setIsSuccess(true); // Show success message
    } catch (error) {
      console.error("Error in handleSubmitNFT:", error);
      toast.error(
        "An unexpected error occurred during submission. Please try again."
      );
    } finally {
      setIsSubmitting(false); // Stop loader
    }
  };

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black bg-opacity-90 p-2">
      <div className="relative bg-[#13172A] text-white rounded-lg w-full max-w-[500px]">
        <button
          onClick={handleClosePopup}
          className="absolute top-3 right-3 text-gray-400 hover:text-gray-200 focus:outline-none z-50"
          aria-label="Close"
        >
          ✕
        </button>

        {!isSuccess && (
          <div className="p-8">
            <h2 className="text-2xl font-bold mb-4">
              Submit and publish your meme
            </h2>

            <p className="text-sm mb-4 text-gray-400">
              Upload media to create a meme that anybody can mint. <br />
              The meme you create will be stored in Walrus powered by SUI.{" "}
              <br />
              Once you hit <strong className="text-white">PUBLISH</strong>, your
              meme will be live.
            </p>

            <div className="bg-gradient-to-r from-[#9B5DFF] via-[#FFB74B] to-[#E72AE0] text-black text-center text-sm font-semibold py-2 px-4 rounded-3xl mb-4">
              ⛽ NFTs stored with <span className="font-bold">Walrus</span>{" "}
              massively decrease gas fees!!
            </div>

            <FileUploader
              onFileUpload={(file, returnedBlobId) =>
                handleFileUpload(file, returnedBlobId)
              }
              onFileRemove={handleFileRemove}
            />
            <p className="text-sm text-gray-400">
              Only one submission per address. <br />
            </p>
            {uploadedFile && (
              <div className="mt-6">
                <label htmlFor="memeTitle" className="block text-sm mb-2">
                  Give Your Meme a Name
                </label>
                <input
                  type="text"
                  id="memeTitle"
                  className="w-full border border-primary_teal bg-primary_dark rounded-xl px-3 py-1.5 text-white"
                  placeholder="Enter meme title..."
                  value={memeTitle}
                  onChange={(e) => {
                    if (e.target.value.length <= 80) {
                      setMemeTitle(e.target.value);
                    } else {
                      toast.error("Maximum 80 characters allowed!");
                    }
                  }}
                />
              </div>
            )}

            <div className="flex justify-end space-x-2 mt-6">
              <button
                onClick={handleClosePopup}
                className="bg-gray-700 text-white text-sm py-2 px-4 rounded-lg hover:bg-gray-600"
              >
                CANCEL
              </button>
              <button
                className={`bg-primary_teal text-black text-sm py-2 px-4 rounded-lg font-bold hover:bg-teal-600 ${
                  !isPublishEnabled && "opacity-50 cursor-not-allowed"
                }`}
                onClick={handleSubmitNFT}
                disabled={!isPublishEnabled || isSubmitting}
              >
                {isSubmitting ? "PUBLISHING..." : "PUBLISH"}
              </button>
            </div>
          </div>
        )}

        {isSuccess && (
          <div className="relative">
            <div
              className="absolute inset-0 pointer-events-none z-10 bg-top bg-repeat-x"
              style={{
                backgroundImage: "url(/Confetti.gif)",
                backgroundSize: "180px auto",
                backgroundPosition: "0 -25px", // Moves the background image higher
                top: "-20px", // Moves the entire div higher
                left: "-20px", // Moves the entire div left
              }}
            ></div>

            <div className="relative text-center px-6 pt-10 pb-5">
              <h2 className="text-3xl font-bold bg-gradient-to-r from-[#17FFFF] via-[#FFB74B] to-[#9B5DFF] text-transparent bg-clip-text mb-6 z-50">
                Your meme is live!
              </h2>

              <div className="max-w-[400px] mx-auto bg-primary_dark rounded-lg p-6">
                <div className="w-[350px] h-[350px] mx-auto flex justify-center items-center mb-4 overflow-hidden">
                  <img
                    src={uploadedFile ? URL.createObjectURL(uploadedFile) : ""}
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
                    title={address || "anonymous"}
                  >
                    @
                    {address && address.length > 20
                      ? `${address.slice(0, 6)}...${address.slice(-4)}`
                      : address || "anonymous"}
                  </p>
                </div>
              </div>

              <div className="flex justify-between space-x-4 mt-3 max-w-[400px] mx-auto">
                <a
                  className="flex-1 flex items-center justify-between bg-[#252B4A] py-1.5 px-4 rounded-lg text-white text-xs"
                  href={`https://walruscan.com/testnet/blob/${blobId}`}
                  target="_blank"
                  rel="noopener noreferrer"
                >
                  <img
                    src="/Sui_Explorer.png"
                    alt="Left Icon"
                    className="w-4 h-4"
                  />
                  View on Walrus Explorer
                  <img
                    src="/Arrow_Up.png"
                    alt="Right Icon"
                    className="w-4 h-4"
                  />
                </a>

                <a
                  className="flex-1 flex items-center justify-between bg-[#252B4A] py-1.5 px-4 rounded-lg text-white text-xs"
                  href={`https://suiscan.xyz/${NETWORK_NAME}/object/${NFTId}`}
                  target="_blank"
                  rel="noopener noreferrer"
                >
                  <img
                    src="/Sui_Scan.png"
                    alt="Left Icon"
                    className="w-4 h-4"
                  />
                  View in SuiScan
                  <img
                    src="/Arrow_Up.png"
                    alt="Right Icon"
                    className="w-4 h-4"
                  />
                </a>
              </div>
            </div>

            <div className="bg-primary_dark  pt-4 pb-6 rounded-b-lg">
              <div className="flex justify-center space-x-2">
                <UnifiedShareButton nftId={memeNftId![0]} size={40} />
              </div>
              <p className="text-sm mt-4 text-center">
                Share with your community! 🔥
              </p>
            </div>
          </div>
        )}
      </div>
    </div>
  );
};

export default SubmitNFT;
