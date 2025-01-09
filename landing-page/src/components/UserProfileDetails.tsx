// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import React from "react";
import UnifiedShareButton from "./UnifiedShareButton";
import { toast } from "react-toastify";
import "react-toastify/dist/ReactToastify.css";

interface UserProfileDetailsProps {
  handle: string;
  nftId?: string;
  profileId?: string;
  hostAvatar?: string;
}

const UserProfileDetails: React.FC<UserProfileDetailsProps> = ({
  handle,
  nftId,
  profileId,
  hostAvatar,
}) => {
  return (
    <div className="flex flex-col sm:flex-row justify-between items-center bg-primary_dark py-6 px-4 rounded-lg space-y-4 sm:space-y-0 sm:space-x-4">
      <div className="flex flex-col sm:flex-row items-center space-y-4 sm:space-y-0 sm:space-x-4">
        <div className="w-16 h-16 rounded-lg bg-[#1C1F2A] flex items-center justify-center">
          <img
            src={hostAvatar || "/default-avatar.png"}
            alt="User Avatar"
            className="rounded-lg w-full h-full"
          />
        </div>
        <div className="text-center sm:text-left">
          <h1 className="text-white font-ppNeueBit text-3xl sm:text-5xl">
            {handle}
          </h1>
          <div className="flex justify-center sm:justify-start items-center space-x-2 mt-2">
            <span className="text-gray-400">
              {profileId!.slice(0, 6)}...{profileId!.slice(-4)}
            </span>
            <button
              onClick={() => {
                navigator.clipboard.writeText(profileId!);
                toast.success("Address copied to clipboard!");
              }}
              className="text-primary_teal"
            >
              <img src="/Copy_Icon2.png" alt="Copy" className="w-4 h-4" />
            </button>
          </div>
        </div>
      </div>
      <UnifiedShareButton nftId={nftId} profileId={profileId} size={40} />
    </div>
  );
};

export default UserProfileDetails;
