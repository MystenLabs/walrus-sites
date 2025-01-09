// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

export const sponsorDeleteTransaction = async (
  url: string,
  body: {
    contestId: string;
    creator: string;
    memeNftId: string;
    token: string;
    userAction: string;
  }
): Promise<{ txBytes: string; txDigest: string }> => {
  try {
    const response = await fetch(`${url}/api/contestUser/delete`, {
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
    throw error;
  }
};
