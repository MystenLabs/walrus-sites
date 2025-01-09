// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//This page will not be used for this current competition
import React from "react";
import Footer from "../components/Footer";
import MinimizedHeader from "../components/MinimizedHeader";
import SimpleNFTContainer from "../components/SimpleNFTContainer";
import UserProfileDetails from "../components/UserProfileDetails";
import { useParams } from "react-router-dom";
// import { useCurrentAccount } from "@mysten/dapp-kit";

const myNFTs = [
  {
    position: 14,
    imageSrc: "/DummyNFT9.png",
    title: "Chillaxin’ Walrus",
    creatorName: "@robot",
    creatorAvatar: "/Host_Icon.png",
    votes: 703,
  },
  {
    position: 15,
    imageSrc: "/DummyNFT10.png",
    title: "SOON!",
    creatorName: "@robot",
    creatorAvatar: "/Host_Icon.png",
    votes: 703,
  },
  {
    position: 16,
    imageSrc: "/DummyNFT6.png",
    title: "CryptoTusk Walrus",
    creatorName: "@robot",
    creatorAvatar: "/Host_Icon.png",
    votes: 703,
  },
  {
    position: 17,
    imageSrc: "/DummyNFT7.png",
    title: "To The Moonrus",
    creatorName: "@robot",
    creatorAvatar: "/Host_Icon.png",
    votes: 703,
  },
  {
    position: 18,
    imageSrc: "/DummyNFT8.png",
    title: "Iceberg Baller",
    creatorName: "@robot",
    creatorAvatar: "/Host_Icon.png",
    votes: 703,
  },
  {
    position: 19,
    imageSrc: "/DummyNFT9.png",
    title: "Chillaxin’ Walrus",
    creatorName: "@robot",
    creatorAvatar: "/Host_Icon.png",
    votes: 703,
  },
  {
    position: 20,
    imageSrc: "/DummyNFT10.png",
    title: "SOON!",
    creatorName: "@robot",
    creatorAvatar: "/Host_Icon.png",
    votes: 703,
  },
];

const ProfilePage: React.FC = () => {
  const { id } = useParams<{ id: string }>(); // Get the id from the URL if present
  return (
    <div className="bg-primary_dark">
      <MinimizedHeader />
      <div className="max-w-[1100px] mx-auto py-6 min-h-screen px-3">
        <UserProfileDetails
          handle="@COOLFISH"
          hostAvatar="/Profile_Avatar.png"
          profileId={id}
        />
        <div className="grid grid-cols-2 sm:grid-cols-2 md:grid-cols-3 xl:grid-cols-5 gap-6 pt-8">
          {myNFTs.map((nft) => (
            <SimpleNFTContainer
              key={nft.position}
              nftId={"1"}
              position={nft.position}
              imageSrc={nft.imageSrc}
              title={nft.title}
              creatorName={nft.creatorName}
              votes={nft.votes}
              onVoteSuccess={() => {}} // Dummy function
            />
          ))}
        </div>
        <Footer />
      </div>
    </div>
  );
};

export default ProfilePage;
