// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { toast } from "react-toastify";

// Uploads a file to Walrus and returns the blob ID
export const uploadFileToWalrus = async (
  url: string,
  uploadedFile: File
): Promise<string> => {
  try {
    const response = await fetch(`${url}/v1/store?epochs=200`, {
      method: "PUT",
      body: uploadedFile,
    });

    if (!response.ok) {
      throw new Error("Failed to upload file to Walrus");
    }

    const storageInfo = await response.json();
    return storageInfo.newlyCreated
      ? storageInfo.newlyCreated.blobObject.blobId
      : storageInfo.alreadyCertified.blobId;
  } catch (error) {
    console.error("Error uploading file:", error);
    toast.error("Failed to upload file. Please try again.");
    throw error;
  }
};

// Sponsors a transaction and returns the transaction details
export const sponsorTransaction = async (
  url: string,
  body: {
    contestId: string;
    memeTitle: string;
    address: string;
    blobId: string;
    token: string;
    userAction: string;
  }
): Promise<{ txBytes: string; txDigest: string }> => {
  try {
    const response = await fetch(`${url}/api/contestUser/mint`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(body),
    });

    if (!response.ok) {
      const errorResponse = await response.json();
      console.error("Sponsor API Error:", errorResponse);
      throw new Error("Failed to sponsor the transaction.");
    }

    return response.json();
  } catch (error) {
    console.error("Error sponsoring transaction:", error);
    toast.error(
      "Failed to sponsor the transaction. Please try again. Remember that each user can only mint one NFT per contest."
    );
    throw error;
  }
};

// Executes the sponsorship and returns the execution digest
export const executeSponsorship = async (
  url: string,
  txDigest: string,
  signature: string
): Promise<string> => {
  try {
    const response = await fetch(`${url}/api/contestUser/executeSponsorship`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        digest: txDigest,
        signature: signature,
      }),
    });

    if (!response.ok) {
      const errorResponse = await response.json();
      console.error("Execute API Error:", errorResponse);
      throw new Error("Failed to execute the transaction.");
    }

    const { digest } = await response.json();
    return digest;
  } catch (error) {
    console.error("Error executing sponsorship:", error);
    toast.error("Failed to execute the transaction. Please try again.");
    throw error;
  }
};
