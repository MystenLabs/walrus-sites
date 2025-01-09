// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import React, { useState } from "react";
import { toast } from "react-toastify";
import { BACKEND_URL, CONTEST_ID } from "../config/globalVariables";
import { useCurrentAccount } from "@mysten/dapp-kit";

interface ReportNFTPopupProps {
  onClose: () => void;
  memeNftId: string;
}

const ReportNFTPopup: React.FC<ReportNFTPopupProps> = ({
  onClose,
  memeNftId,
}) => {
  const account = useCurrentAccount();
  const [selectedIssue, setSelectedIssue] = useState<string | null>(null);

  const issues = [
    {
      id: "violates_rules",
      title: "Violates Contest Rules",
      description: "Breaches the guidelines or terms of the competition.",
    },
    {
      id: "inappropriate_content",
      title: "Inappropriate Content",
      description: "Contains offensive, explicit, or NSFW material.",
    },
    {
      id: "copyright_infringement",
      title: "Copyright Infringement",
      description: "Uses content without proper ownership or rights.",
    },
    {
      id: "spam_or_scams",
      title: "Spam or Scams",
      description: "Promotes fraudulent or misleading information.",
    },
    {
      id: "hate_speech",
      title: "Hate Speech or Discrimination",
      description: "Includes harmful, abusive, or hateful language.",
    },
  ];

  const handleSubmitReport = async () => {
    if (!selectedIssue) {
      toast.error("Please select an issue to report.");
      return;
    }
    if (!account) {
      toast.error("Please connect your wallet to report.");
      return;
    }

    try {
      const response = await fetch(`${BACKEND_URL}/api/contestUser/report`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          contestId: CONTEST_ID,
          memeNftId,
          reporter: account.address,
          text: selectedIssue,
        }),
      });

      if (!response.ok) {
        const { reason } = await response.json();
        toast.error(reason || "Failed to submit report.");
        return;
      }

      toast.success("Report submitted successfully!");
      onClose(); // Close the popup after success
    } catch (error) {
      console.error("Error submitting report:", error);
      toast.error(
        "An error occurred while submitting the report. Please try again."
      );
    }
  };

  return (
    <div className="fixed inset-0 flex items-center justify-center z-50 p-2">
      <div
        className="absolute inset-0 bg-black bg-opacity-70"
        onClick={onClose}
      ></div>

      <div className="relative bg-[#1E233B] text-white rounded-lg shadow-lg p-6 max-w-lg w-full z-50 ">
        <button
          onClick={onClose}
          className="absolute top-3 right-3 text-gray-400 hover:text-gray-200 focus:outline-none"
        >
          ✕
        </button>

        <h2 className="text-2xl font-bold mb-4">Report NFT</h2>
        <p className="text-xl font-bold mb-4">What issue are you reporting?</p>

        <div className="space-y-4">
          {issues.map((issue) => (
            <div
              key={issue.id}
              onClick={() => setSelectedIssue(issue.id)}
              className={`cursor-pointer p-4 rounded-lg border ${
                selectedIssue === issue.id
                  ? "border-primary_teal bg-[#252B4A]"
                  : "border-[#2D344D] bg-transparent"
              }`}
            >
              <h3 className="font-semibold text-base">{issue.title}</h3>
              <p className="text-sm text-gray-400">{issue.description}</p>
            </div>
          ))}
        </div>
        <div className="flex justify-end mt-6">
          <button
            onClick={handleSubmitReport}
            className="bg-primary_teal text-black text-base font-semibold py-2 px-4 rounded-lg hover:bg-[#00FFC2]"
          >
            SUBMIT REPORT
          </button>
        </div>
      </div>
    </div>
  );
};

export default ReportNFTPopup;
