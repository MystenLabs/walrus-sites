// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import React, { useEffect, useState, useRef } from "react";
import Header from "../components/Header";
import MinimizedHeader from "../components/MinimizedHeader";
import Top2VotedNFTContainer from "../components/Top2VotedNFTContainer";
import Top10VotedNFTContainer from "../components/Top10VotedNFTContainer";
import Footer from "../components/Footer";
import SimpleNFTContainer from "../components/SimpleNFTContainer";
import { useLocation, useNavigate, useParams } from "react-router-dom";
import MemeDetails from "../components/MemeDetails";
import { useCurrentAccount, useSuiClient } from "@mysten/dapp-kit";
import { BACKEND_URL, CONTEST_ID, PACKAGE_ID } from "../config/globalVariables";
import { getContestObject } from "../helpers/getContestObject";
import UserVoteContainer from "../components/UserVoteContainer";
import { SuiClient, SuiMoveObject, SuiParsedData } from "@mysten/sui/client";

interface MemeNftOnChain {
  id: {
    id: string;
  };
  title: string;
  creator: string;
  blob_id: string;
  contest_id: string;
}

interface MemeWithVotes extends MemeNftOnChain {
  votes: number;
}

const HomePage: React.FC = () => {
  const account = useCurrentAccount();
  const [NFTs, setNFTs] = useState<MemeWithVotes[]>([]);
  const [allNFTs, setAllNFTs] = useState<MemeWithVotes[]>([]);
  const [page, setPage] = useState(1);
  const [isLoading, setIsLoading] = useState(false);
  const [isPopupOpen, setIsPopupOpen] = useState(false);
  const [popupId, setPopupId] = useState<string | null>(null);
  const [isHeaderVisible, setIsHeaderVisible] = useState(true);
  const [sortingOption, setSortingOption] = useState<"votes" | "createdAt">(
    "votes"
  );
  const [sortingOptionsPopupOpen, setSortingOptionsPopupOpen] = useState(false);
  const { id } = useParams<{ id: string }>();
  const location = useLocation();
  const navigate = useNavigate();
  const [refreshKey, setRefreshKey] = useState(0);
  const [endDate, setEndDate] = useState(0);
  const [submissionsCount, setSubmissionsCount] = useState(0);
  const [myVotes, setMyVotes] = useState<{ nftId: string }[]>([]);
  const suiClient = useSuiClient();

  const host = "@walruscoach";
  const hostAvatar = "/Host_Icon.png";
  const headerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const fetchContestDetails = async () => {
      try {
        const contestObj = await getContestObject({
          suiClient: suiClient as unknown as SuiClient,
          contestId: CONTEST_ID,
        });
        setEndDate(contestObj.end_time);
      } catch (error) {
        console.error("Error fetching contest details:", error);
      }
    };

    fetchContestDetails();
  }, [suiClient]);

  useEffect(() => {
    const fetchNFTs = async () => {
      try {
        setIsLoading(true);
        const response = await fetch(
          `${BACKEND_URL}/api/contestUser/nfts?page=${page}&limit=10&${sortingOption}=desc`
        );
        const data = await response.json();

        const nftIds = data.map((item: { id: string }) => item.id);

        const nftObjects = await suiClient
          .multiGetObjects({
            ids: nftIds,
            options: { showContent: true },
          })
          .then((res) =>
            res.map(({ data }) => {
              const content = data?.content as Extract<
                SuiParsedData,
                { dataType: "moveObject" }
              >;
              return content?.fields as unknown as MemeNftOnChain;
            })
          );

        // Fetch votes for the NFTs
        const votesResponse = await fetch(
          `${BACKEND_URL}/api/contestUser/nftVotes`,
          {
            method: "POST",
            headers: {
              "Content-Type": "application/json",
            },
            body: JSON.stringify({ nftIds }),
          }
        );

        const votesData: { id: string; votes: number }[] =
          await votesResponse.json();

        // Combining the NFT data with votes
        const nftsWithVotes: MemeWithVotes[] = nftObjects.map((nft) => {
          const voteData = votesData.find((vote) => vote.id === nft.id.id);
          return {
            ...nft,
            votes: voteData?.votes || 0, // Default to 0 if not found
          };
        });

        if (page === 1) {
          setAllNFTs(nftsWithVotes);
        } else {
          setAllNFTs((prevNFTs) => [...prevNFTs, ...nftsWithVotes]);
        }

        setNFTs(nftsWithVotes);
      } catch (error) {
        console.error("Error fetching NFTs:", error);
      } finally {
        setIsLoading(false);
      }
    };

    fetchNFTs();
  }, [page, sortingOption, refreshKey]);

  useEffect(() => {
    const fetchSubmissionCount = async () => {
      try {
        const response = await fetch(`${BACKEND_URL}/api/contestUser/nftCount`);
        if (!response.ok) {
          throw new Error("Failed to fetch submission count");
        }
        const data = await response.json();
        setSubmissionsCount(data.count);
      } catch (error) {
        console.error("Error fetching submission count:", error);
      }
    };

    fetchSubmissionCount();
  }, [refreshKey]);

  useEffect(() => {
    if (location.pathname.startsWith("/meme-details/") && id) {
      setPopupId(id);
      setIsPopupOpen(true);
    } else {
      setIsPopupOpen(false);
      setPopupId(null);
    }
  }, [id, location.pathname]);

  const fetchMyVotes = async (suiClient: SuiClient, userAddress: string) => {
    try {
      const voteProofResponse = await suiClient.getOwnedObjects({
        owner: userAddress,
        filter: {
          StructType: `${PACKAGE_ID}::vote_proof::VoteProof`,
        },
      });

      const voteProofObjects = voteProofResponse?.data || [];

      if (voteProofObjects.length === 0) {
        console.log("No votes found for the user.");
        setMyVotes([]);
        return;
      }

      const nftIds = await Promise.all(
        voteProofObjects.map(async (obj) => {
          const objectId = obj?.data?.objectId;
          if (!objectId) return null;

          const res = await suiClient.getObject({
            id: objectId,
            options: { showContent: true },
          });

          const content = res?.data?.content as SuiMoveObject | undefined;
          const fields = content?.fields as { nft_id: string } | undefined;

          return fields?.nft_id || null;
        })
      );

      const validNftIds = nftIds.filter((nftId): nftId is string => !!nftId);

      if (validNftIds.length === 0) {
        console.log("No valid NFT IDs found.");
        setMyVotes([]);
        return;
      }

      const nftObjects = await suiClient
        .multiGetObjects({
          ids: validNftIds,
          options: { showContent: true },
        })
        .then((res) =>
          res.map(({ data }) => {
            const content = data?.content as SuiMoveObject | undefined;
            return content?.fields as MemeNftOnChain | undefined;
          })
        );

      // Filter NFTs based on the current contest ID
      const filteredNftObjects = nftObjects.filter(
        (nft): nft is MemeNftOnChain => !!nft && nft.contest_id === CONTEST_ID
      );

      setMyVotes(
        filteredNftObjects.map((nft) => ({
          nftId: nft.id.id,
          title: nft.title,
        }))
      );
    } catch (error) {
      console.error("Failed to load votes:", error);
      setMyVotes([]);
    }
  };

  useEffect(() => {
    const fetchVotes = async () => {
      if (!account?.address) return;
      await fetchMyVotes(suiClient as unknown as SuiClient, account.address);
    };

    fetchVotes();
  }, [refreshKey, account, suiClient]);

  useEffect(() => {
    const observer = new IntersectionObserver(
      ([entry]) => {
        setIsHeaderVisible(entry.isIntersecting);
      },
      { threshold: 0.1 }
    );

    if (headerRef.current) {
      observer.observe(headerRef.current);
    }

    return () => {
      if (headerRef.current) {
        observer.unobserve(headerRef.current);
      }
    };
  }, []);

  const handleClosePopup = () => {
    setIsPopupOpen(false);
    setPopupId(null);
    navigate("/");
  };

  const handleNextPage = () => {
    setPage((prevPage) => prevPage + 1);
  };

  const handlePreviousPage = () => {
    if (page > 1) {
      setPage((prevPage) => prevPage - 1);
    }
  };

  return (
    <>
      {!isHeaderVisible && <MinimizedHeader />}
      <div className="min-h-screen flex flex-col items-center bg-primary_dark custom_lg:p-8 md:p-6 sm:p-4 p-2 font-ppNeueMontreal">
        <div ref={headerRef}>
          <Header
            endDate={endDate}
            submissionsCount={submissionsCount}
            host={host}
            hostAvatar={hostAvatar}
            onSubmitSuccess={() => setRefreshKey((prev) => prev + 1)}
          />
        </div>
        <div className="max-w-[1100px] mx-auto pt-6 lg:min-w-[800px]">
          <div className="flex justify-between items-center mb-6 relative">
            {myVotes.length && (
              <h1 className="text-white text-3xl font-semibold pt-6 pl-2">
                Your votes{" "}
                <span className="text-gray-500 text-sm pl-1">
                  ({3 - myVotes.length} remaining)
                </span>
              </h1>
            )}
          </div>

          {myVotes.length && (
            <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-6 pb-10">
              {myVotes.map((vote, index) => (
                <UserVoteContainer key={index} nftId={vote.nftId} />
              ))}
            </div>
          )}

          <div className="flex justify-end items-center ">
            <div className="relative py-3">
              <button
                onClick={() =>
                  setSortingOptionsPopupOpen(!sortingOptionsPopupOpen)
                }
                className="text-primary_teal  border border-primary_teal px-4 py-2 rounded-lg hover:bg-[#97F0E54D] focus:outline-none text-sm flex items-center"
              >
                <span className="pr-3 text-sm">
                  SORT BY:{"  "}
                  {sortingOption === "votes" ? "TOP VOTED" : "MOST RECENT"}{" "}
                </span>
                <img
                  src="/Arrow_Down.png"
                  alt="Arrow Down"
                  className="w-2 h-1.5"
                />
              </button>

              {sortingOptionsPopupOpen && (
                <ul className="absolute right-0 mt-2 w-48 bg-[#161A30] border border-gray-700 rounded-lg shadow-lg z-10">
                  <li
                    onClick={() => {
                      setSortingOption("votes");
                      setPage(1);
                      setSortingOptionsPopupOpen(false);
                    }}
                    className="px-4 py-2 text-white hover:bg-gray-700 cursor-pointer"
                  >
                    TOP VOTED
                  </li>
                  <li
                    onClick={() => {
                      setSortingOption("createdAt");
                      setPage(1);
                      setSortingOptionsPopupOpen(false);
                    }}
                    className="px-4 py-2 text-white hover:bg-gray-700 cursor-pointer"
                  >
                    MOST RECENT
                  </li>
                </ul>
              )}
            </div>
          </div>
          <h1 className="text-white text-3xl font-semibold pt-3 mb-6 pl-2">
            {submissionsCount} Meme NFTs
          </h1>
          {page === 1 && sortingOption === "votes" ? (
            <>
              <div className="grid grid-cols-1 sm:grid-cols-2 custom_lg:gap-6 lg:gap-6 md:gap-10 sm:gap-8 gap-2">
                {allNFTs.slice(0, 2).map((nft, index) => (
                  <Top2VotedNFTContainer
                    key={nft.id.id}
                    nftId={nft.id.id}
                    position={index + 1}
                    imageSrc={
                      `https://aggregator.walrus-testnet.walrus.space/v1/${nft.blob_id}` ||
                      "/NotFound.png"
                    }
                    title={nft.title}
                    creatorName={nft.creator}
                    votes={nft.votes}
                    onVoteSuccess={() => setRefreshKey((prev) => prev + 1)}
                  />
                ))}
              </div>

              <div className="grid grid-cols-1 sm:grid-cols-2 custom_lg:grid-cols-4 gap-6 pt-10">
                {allNFTs.slice(2, 10).map((nft, index) => (
                  <Top10VotedNFTContainer
                    key={nft.id.id}
                    nftId={nft.id.id}
                    position={index + 3}
                    imageSrc={
                      `https://aggregator.walrus-testnet.walrus.space/v1/${nft.blob_id}` ||
                      "/NotFound.png"
                    }
                    title={nft.title}
                    creatorName={nft.creator}
                    votes={nft.votes}
                    onVoteSuccess={() => setRefreshKey((prev) => prev + 1)}
                  />
                ))}
              </div>
            </>
          ) : (
            <div className="grid grid-cols-2 sm:grid-cols-2 md:grid-cols-3 xl:grid-cols-5 gap-6 pt-8">
              {allNFTs.slice((page - 1) * 10, page * 10).map((nft) => (
                <SimpleNFTContainer
                  key={nft.id.id}
                  nftId={nft.id.id}
                  position={allNFTs.indexOf(nft)}
                  imageSrc={`https://aggregator.walrus-testnet.walrus.space/v1/${nft.blob_id}`}
                  title={nft.title}
                  creatorName={nft.creator}
                  votes={nft.votes}
                  onVoteSuccess={() => setRefreshKey((prev) => prev + 1)}
                />
              ))}
            </div>
          )}

          <div className="flex justify-between mt-20 mb-5">
            <button
              className="bg-gray-600 text-white py-2 px-4 rounded hover:bg-gray-700 disabled:opacity-50"
              onClick={handlePreviousPage}
              disabled={page === 1}
            >
              Previous
            </button>
            <button
              className="bg-gray-600 text-white py-2 px-4 rounded hover:bg-gray-700 disabled:opacity-50"
              onClick={handleNextPage}
              disabled={NFTs.length < 10 || isLoading}
            >
              Next
            </button>
          </div>
        </div>
        <Footer />
        <MemeDetails
          isOpen={isPopupOpen}
          onClose={handleClosePopup}
          nftId={popupId}
          onVoteSuccess={() => setRefreshKey((prev) => prev + 1)}
        />
      </div>
    </>
  );
};

export default HomePage;
