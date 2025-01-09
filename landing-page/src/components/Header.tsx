// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import React, { useEffect, useState } from "react";
import "@mysten/dapp-kit/dist/index.css";
import { ConnectButton, useSuiClient } from "@mysten/dapp-kit";
import SubmitNFT from "./SubmitNFT";
import { useCurrentAccount } from "@mysten/dapp-kit";
import { toast } from "react-toastify";
import "react-toastify/dist/ReactToastify.css";
import { CONTEST_ID } from "../config/globalVariables";
import { getMemeNftObject } from "../helpers/getMemeNFTObject";
import { useNavigate } from "react-router-dom";
import { SuiClient } from "@mysten/sui/client";
import { getMemeNftId } from "../helpers/getMemeNFTId";

interface HeaderProps {
  endDate: number;
  submissionsCount: number;
  host: string;
  hostAvatar: string;
  onSubmitSuccess?: () => void;
}
interface MemeNftOnChain {
  id: {
    id: string;
  };
  title: string;
  creator: string;
  blob_id: string;
  contest_id: string;
}

const Header: React.FC<HeaderProps> = ({
  endDate,
  submissionsCount,
  host,
  hostAvatar,
  onSubmitSuccess,
}) => {
  const navigate = useNavigate();
  const [isSubmitNFTOpen, setIsSubmitNFTOpen] = useState(false);
  const [submittedNFT, setSubmittedNFT] = useState<MemeNftOnChain | null>(null); // Holds the user's submission if any
  const account = useCurrentAccount();
  const [timeLeft, setTimeLeft] = useState("");
  const suiClient = useSuiClient();

  const calculateTimeLeft = () => {
    const now = new Date().getTime();
    const difference = endDate - now;

    if (difference > 0) {
      const days = Math.floor(difference / (1000 * 60 * 60 * 24));
      const hours = Math.floor((difference / (1000 * 60 * 60)) % 24);
      const minutes = Math.floor((difference / (1000 * 60)) % 60);
      const seconds = Math.floor((difference / 1000) % 60);

      return `${days} Days ${hours} Hrs ${minutes} Mins ${seconds} Secs`;
    }

    return "Contest Ended";
  };

  useEffect(() => {
    const timer = setInterval(() => {
      setTimeLeft(calculateTimeLeft());
    }, 1000);

    return () => clearInterval(timer); // Cleanup interval on component unmount
  }, [endDate]);

  const handleOpenSubmitNFT = () => {
    if (account?.address === undefined) {
      toast.error("Please connect your wallet to submit a meme.");
      return;
    }

    setIsSubmitNFTOpen(true);
  };
  const handleCloseSubmitNFT = () => setIsSubmitNFTOpen(false);
  const handleClick = () => {
    if (submittedNFT?.id) {
      navigate(`/meme-details/${submittedNFT.id.id}`);
    }
  };

  useEffect(() => {
    const fetchSubmittedNFT = async () => {
      if (!account?.address) return;

      try {
        // Fetch all MemeNFT IDs owned by the user
        const memeNftIds = await getMemeNftId({
          suiClient: suiClient as unknown as SuiClient,
          address: account.address,
        });

        if (!memeNftIds || memeNftIds.length === 0) {
          setSubmittedNFT(null);
          return;
        }

        // Fetch details for each MemeNFT object
        const nftObjects = await Promise.all(
          memeNftIds.map((nftId) =>
            getMemeNftObject(nftId, suiClient as unknown as SuiClient)
          )
        );

        // Filter out null results and find the one matching the current contest ID
        const userSubmission = nftObjects
          .filter(
            (nftObject): nftObject is MemeNftOnChain => nftObject !== null
          )
          .find((nftObject) => nftObject?.contest_id === CONTEST_ID);

        // Update state with the found submission or null if no match
        setSubmittedNFT(userSubmission || null);
      } catch (error) {
        console.error("Error fetching submitted NFT:", error);
      }
    };

    fetchSubmittedNFT();
  }, [account, onSubmitSuccess]);

  const [buttonStyle, setButtonStyle] = useState({
    backgroundColor: "#C684F6",
    color: "black",
    cursor: "pointer",
    padding: "12px 24px",
    fontSize: "18px",
  });
  useEffect(() => {
    const handleResize = () => {
      if (window.innerWidth >= 1100) {
        setButtonStyle({
          backgroundColor: "#C684F6",
          color: "black",
          cursor: "pointer",
          padding: "12px 24px",
          fontSize: "18px",
        });
      } else if (window.innerWidth >= 768) {
        setButtonStyle({
          backgroundColor: "#C684F6",
          color: "black",
          cursor: "pointer",
          padding: "10px 20px",
          fontSize: "16px",
        });
      } else if (window.innerWidth >= 400) {
        setButtonStyle({
          backgroundColor: "#C684F6",
          color: "black",
          cursor: "pointer",
          padding: "8px 16px",
          fontSize: "15px",
        });
      } else {
        setButtonStyle({
          backgroundColor: "#C684F6",
          color: "black",
          cursor: "pointer",
          padding: "8px 12px",
          fontSize: "12px",
        });
      }
    };

    handleResize(); // Call once to set the initial style
    window.addEventListener("resize", handleResize);

    return () => {
      window.removeEventListener("resize", handleResize);
    };
  }, []);
  return (
    <div className="relative p-0.5 rounded-3xl bg-gradient-to-t from-gradientStart to-gradientEnd max-w-[1300px] flex items-center justify-center">
      <header
        className="bg-primary_dark text-white p-6 rounded-3xl max-w-[1292px] w-full"
        style={{
          backgroundImage: "url('/Tiles_Bg.png')",
          backgroundSize: "cover",
          backgroundRepeat: "no-repeat",
          backgroundPosition: "bottom",
        }}
      >
        <div className="flex justify-between items-center lg:px-2 px-0">
          <img
            src="/logo.png"
            alt="Walrus Adventure"
            className="custom_lg:w-[280px] md:w-[220px] w-[160px] h-auto"
          />
          <ConnectButton connectText="CONNECT WALLET" style={buttonStyle} />
        </div>
        <div className="relative custom_lg:px-8 custom_lg:py-8 md:py-5 md:px-0 py-3 px-0 flex flex-col md:flex-row space-y-8 md:space-y-0 md:space-x-8">
          <div className="flex-1">
            <h2 className="custom_lg:text-5xl font-bold mb-2 font-ppMondwest md:text-5xl text-4xl ">
              Make a Meme with Walrus
            </h2>
            <div className="flex flex-wrap items-center custom_lg:space-x-12 md:space-x-8 space-x-6">
              <div className="flex items-center space-x-2">
                <img
                  src={hostAvatar}
                  alt="host avatar"
                  className="w-8 h-8 rounded-full"
                />
                <div>
                  <p className="text-xs text-gray-500">Hosted by</p>
                  <p>{host}</p>
                </div>
              </div>
              <div className="text-left mx-2 ">
                <p className="text-xs text-gray-500">Submissions</p>
                <p>{submissionsCount}</p>
              </div>
              <div className="text-right ml-2 hidden sm:block">
                <p className="text-accent font-ppMondwest text-[23px]">
                  {timeLeft}
                </p>
              </div>
            </div>
            <p className="text-accent font-ppMondwest text-[23px] sm:hidden block pt-2">
              {timeLeft}
            </p>
            <p className="text-lg font-ppNeueBit py-5 custom_lg:w-[65%] md:w-[80%]">
              It’s about to be a new year, and Walrus is getting ready for big
              things in 2025! 🦭💫  What adventures lie ahead? Tell us with
              memes! Make us laugh, gasp, or get rowdy with your creative
              takes.  Place walruses in all kinds of wild scenarios—from
              conquering the blockchain seas to finding a bestie in the
              metaverse. Set your imagination free and put your meme skills to
              the test. Let the Walrus adventure begin! 🐾💥
            </p>

            <div className="flex flex-col sm:flex-row items-center sm:items-start space-y-4 sm:space-y-0 sm:space-x-4 mt-2">
              <div className="flex justify-center w-full sm:w-auto">
                {submittedNFT ? (
                  <div
                    className="bg-primary_teal text-black font-bold py-3 px-3 rounded-lg space-x-2 max-w-[400px] w-full flex items-center hover:cursor-pointer"
                    onClick={handleClick}
                  >
                    <img
                      src={`https://aggregator.walrus-testnet.walrus.space/v1/${submittedNFT.blob_id}`}
                      alt="Submission Thumbnail"
                      className="w-16 h-16 rounded-lg"
                    />
                    <div className="pl-2 pr-10">
                      <p className="text-sm text-[#4A847D]">YOUR SUBMISSION</p>
                      <p className="text-xl font-bold">
                        {submittedNFT.title || "Untitled Meme"}
                      </p>
                    </div>
                    <img
                      src="/Arrow_Right.png"
                      alt="Arrow right"
                      className="w-6 h-6"
                    />
                  </div>
                ) : (
                  <button
                    onClick={handleOpenSubmitNFT}
                    className="flex items-center justify-center bg-primary_teal text-black font-bold py-3 px-6 rounded-lg space-x-2 max-w-[300px] w-full"
                  >
                    <img
                      src="/Plus_Icon.png"
                      alt="Plus Icon"
                      className="w-4 h-4"
                    />
                    <span className="align-middle font-ppNeueMontrealBold">
                      SUBMIT YOUR MEME
                    </span>
                  </button>
                )}
              </div>
              <a
                href="/terms-of-service"
                className="text-accent underline custom_lg:text-2xl text-xl font-ppNeueBit custom_lg:pl-5 pl-2 pt-0 sm:pt-0"
                style={{ alignSelf: "center" }}
              >
                Contest Terms & Conditions
              </a>
            </div>
          </div>

          <div className="absolute custom_lg:bottom-[-24px] custom_lg:right-[-24px] md:bottom-[-24px] md:right-[-24px] hidden md:block">
            <img
              src="/Walrus_On_Floatie.png"
              alt="Walrus illustration"
              className="custom_lg:w-[500px] md:w-[300px] w-[250px] h-auto rounded-br-3xl"
            />
          </div>
        </div>
      </header>
      <SubmitNFT
        isOpen={isSubmitNFTOpen}
        onClose={handleCloseSubmitNFT}
        address={account?.address}
        onSubmitSuccess={onSubmitSuccess}
      />
    </div>
  );
};

export default Header;
