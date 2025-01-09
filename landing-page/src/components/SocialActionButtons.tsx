// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import React, { useState } from "react";
import { toast } from "react-toastify";
import "react-toastify/dist/ReactToastify.css";

interface SocialActionButtonProps {
  nftId?: string;
  profileId?: string;
  size?: number;
}

const SocialActionButtons: React.FC<SocialActionButtonProps> = ({
  nftId,
  profileId,
  size = 40, // Default size is 40px
}) => {
  const [copyState, setCopyState] = useState(false); // Track copy state
  const iconSize = size * 0.5; // Icon size relative to button size
  const spacing = size * 0.2; // Spacing relative to button size

  // Determine the shareable URL based on whether nftId or profileId is provided
  const shareUrl = nftId
    ? `https://walrusadventure.walrus.site/meme-details/${nftId}`
    : `https://walrusadventure.walrus.site/profile/${profileId}`;

  const handleCopyToClipboard = () => {
    navigator.clipboard.writeText(shareUrl);
    toast.success("Copied to clipboard!");
    setCopyState(true);
    setTimeout(() => setCopyState(false), 3000); // Reset after 3 seconds
  };

  const handleTwitterShare = () => {
    const twitterUrl = `https://twitter.com/intent/tweet?text=Check%20out%20this%20content!%20${encodeURIComponent(
      shareUrl
    )}`;
    window.open(twitterUrl, "_blank", "noopener,noreferrer");
  };

  const handleTelegramShare = () => {
    const telegramUrl = `https://t.me/share/url?url=${encodeURIComponent(
      shareUrl
    )}&text=Check%20out%20this%20content!`;
    window.open(telegramUrl, "_blank", "noopener,noreferrer");
  };

  const handleDiscordShare = () => {
    const discordUrl = "https://discord.com/";
    window.open(discordUrl, "_blank", "noopener,noreferrer");
  };

  return (
    <div
      className="flex justify-center"
      style={{
        gap: `${spacing}px`, // Apply spacing dynamically
      }}
    >
      <button
        onClick={handleTwitterShare}
        className="bg-[#252B4A] text-white rounded-lg hover:bg-gray-600 flex items-center justify-center"
        style={{
          width: `${size}px`,
          height: `${size}px`,
        }}
      >
        <img
          src="/x_logo.png"
          alt="Twitter"
          style={{
            width: `${iconSize}px`,
            height: `${iconSize}px`,
          }}
        />
      </button>
      <button
        onClick={handleDiscordShare}
        className="bg-[#252B4A] text-white rounded-lg hover:bg-gray-600 flex items-center justify-center"
        style={{
          width: `${size}px`,
          height: `${size}px`,
        }}
      >
        <img
          src="/discord_logo.png"
          alt="Discord"
          style={{
            width: `${iconSize}px`,
            height: `${iconSize}px`,
          }}
        />
      </button>
      <button
        onClick={handleTelegramShare}
        className="bg-[#252B4A] text-white rounded-lg hover:bg-gray-600 flex items-center justify-center"
        style={{
          width: `${size}px`,
          height: `${size}px`,
        }}
      >
        <img
          src="/telegram_logo.png"
          alt="Telegram"
          style={{
            width: `${iconSize}px`,
            height: `${iconSize}px`,
          }}
        />
      </button>
      <button
        onClick={handleCopyToClipboard}
        className={`flex items-center justify-center rounded-lg ${
          copyState ? "bg-primary_teal text-black" : "bg-[#252B4A] text-white"
        } hover:bg-gray-600 px-3`}
        style={{
          height: `${size}px`,
        }}
      >
        {copyState ? (
          <span className="text-xs font-bold">✓ Copied</span>
        ) : (
          <>
            <img
              src="/Copy_Icon.png"
              alt="Copy Link"
              style={{
                width: `${iconSize}px`,
                height: `${iconSize}px`,
              }}
            />
            <span className="text-xs ml-2">Copy Link</span>
          </>
        )}
      </button>
    </div>
  );
};

export default SocialActionButtons;
