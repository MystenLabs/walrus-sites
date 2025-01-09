// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { SuiClient } from "@mysten/sui/client";
import { toast } from "react-toastify";
import { BACKEND_URL } from "../config/globalVariables";

// Step 1: Validate reCAPTCHA and get token
export const getReCaptchaToken = async (
  handleReCaptchaVerify: () => Promise<string | null>
) => {
  const token = await handleReCaptchaVerify();
  if (!token) {
    toast.error("reCAPTCHA validation failed. Please try again.");
    throw new Error("reCAPTCHA validation failed");
  }
  return token;
};

// Step 2: Sponsor transaction
export const sendSponsorRequest = async (
  contestId: string,
  nftId: string,
  voterAddress: string,
  token: string
) => {
  const response = await fetch(`${BACKEND_URL}/api/contestUser/vote`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      contestId,
      memeNftId: nftId,
      voterAddress,
      token,
      userAction: "vote_nft",
    }),
  });

  if (!response.ok) {
    const errorResponse = await response.json();
    console.error("Sponsor API Error:", errorResponse);
    toast.error(
      "Failed to initiate voting. Remember, you can only vote 3 times."
    );
    throw new Error("Sponsor request failed");
  }

  return await response.json();
};

// Step 3: Sign the transaction
export const signTransaction = async (
  signTransactionBlock: any,
  sponsoredBytes: string
) => {
  try {
    const { signature } = await signTransactionBlock({
      transaction: sponsoredBytes as any,
    });

    if (!signature) {
      throw new Error("Signature generation failed");
    }

    return signature;
  } catch (error) {
    toast.error("Failed to sign the transaction. Please try again.");
    console.error("Error during signing transaction:", error);
    throw error;
  }
};

// Step 4: Execute sponsorship
export const executeSponsorshipTransaction = async (
  txDigest: string,
  signature: string
) => {
  const response = await fetch(
    `${BACKEND_URL}/api/contestUser/executeSponsorship`,
    {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ digest: txDigest, signature }),
    }
  );

  if (!response.ok) {
    const errorResponse = await response.json();
    console.error("Execute API Error:", errorResponse);
    toast.error("Failed to execute the transaction. Please try again.");
    throw new Error("Sponsorship execution failed");
  }

  return await response.json();
};

// Step 5: Wait for transaction completion
export const waitForTransactionCompletion = async (
  suiClient: SuiClient,
  executedDigest: string
) => {
  await suiClient.waitForTransaction({
    digest: executedDigest,
    timeout: 10_000,
  });
};

// Step 6: Fetch transaction details
export const fetchTransactionDetails = async (
  suiClient: SuiClient,
  executedDigest: string
) => {
  const transactionResult = await suiClient.getTransactionBlock({
    digest: executedDigest,
    options: {
      showEffects: true,
      showObjectChanges: true,
    },
  });

  if (transactionResult.effects?.status?.status !== "success") {
    toast.error("Transaction failed. Please try again.");
    console.error("Transaction effects:", transactionResult.effects);
    throw new Error("Transaction failed");
  }

  console.log("Transaction successful:", transactionResult);
};
