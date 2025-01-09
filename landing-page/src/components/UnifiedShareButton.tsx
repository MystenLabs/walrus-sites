// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import React from "react";
import SocialActionButtons from "./SocialActionButtons";

interface UnifiedShareButtonProps {
  nftId?: string;
  profileId?: string;
  size?: number; // Button size
}

const UnifiedShareButton: React.FC<UnifiedShareButtonProps> = ({
  nftId,
  profileId,
  size,
}) => {
  const handleShare = async () => {
    const shareUrl = nftId
      ? `https://walrusadventure.walrus.site/meme-details/${nftId}`
      : profileId
      ? `https://walrusadventure.walrus.site/profile/${profileId}`
      : undefined;

    if (!shareUrl) {
      console.error("No valid URL to share.");
      return;
    }

    if (navigator.share) {
      try {
        await navigator.share({
          title: nftId
            ? "Check out this meme!"
            : "Check out this profile on Walrus Adventure!",
          text: nftId
            ? "I found this submission on Walrus Adventure!"
            : "I found this awesome profile on Walrus Adventure",
          url: shareUrl,
        });
        console.log("Content shared successfully!");
      } catch (error) {
        console.error("Error sharing content:", error);
      }
    } else {
      console.log("Web Share API is not supported in this browser.");
    }
  };

  // Render fallback using `SocialActionButtons` if Web Share API is not supported
  if (!navigator.share) {
    return (
      <SocialActionButtons nftId={nftId} profileId={profileId} size={size} />
    );
  }

  return (
    <button
      onClick={handleShare}
      className="bg-[#252B4A] text-white rounded-lg hover:bg-gray-600 flex items-center justify-center space-x-3 px-2.5 py-1.5 "
    >
      <img src="/x_logo.png" alt="Share" className="w-4 h-4" />
      <img src="/discord_logo.png" alt="Share" className="w-4 h-4" />
      <img src="/telegram_logo.png" alt="Share" className="w-4 h-4" />
      <span>Share</span>
    </button>
  );
};

export default UnifiedShareButton;
