// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import React from "react";
import Footer from "../components/Footer";
import MinimizedHeader from "../components/MinimizedHeader";

const ToSPage: React.FC = () => {
  return (
    <div className="bg-primary_dark font-ppNeueMontreal">
      <MinimizedHeader />
      <div className="min-h-screen flex flex-col items-center bg-primary_dark py-10 font-ppNeueMontreal">
        <div className="w-full max-w-[1100px] px-6 sm:px-10 text-white">
          <h1 className="text-4xl sm:text-5xl font-bold mb-4 text-left font-ppNeueMontrealBold">
            Terms & Conditions
          </h1>
          <p className="text-sm  mb-16 text-left">
            <span className="font-bold">Last Updated:</span> Sep 18, 2024
          </p>
          <h2 className="text-3xl sm:text-3xl font-bold mb-4 text-left">
            Welcome to Walrus Adventure
          </h2>
          <p className="text-base leading-relaxed mb-6 text-left font-ppNeueMontrealThin">
            Welcome to the Walrus Adventure NFT Meme Contest (the “Contest”),
            operated using Walrus Technology on the SUI blockchain. By
            participating, you agree to comply with these Terms and Conditions.
            If you do not agree, please refrain from participating.
          </p>
          <p className="text-base leading-relaxed mb-8 text-left font-ppNeueMontrealThin">
            IMPORTANT NOTICE REGARDING ARBITRATION FOR U.S. CUSTOMERS: WHEN YOU
            AGREE TO THESE TERMS YOU ARE AGREEING (WITH LIMITED EXCEPTION) TO
            RESOLVE ANY DISPUTE BETWEEN YOU AND SUI THROUGH BINDING, INDIVIDUAL
            ARBITRATION RATHER THAN IN COURT. PLEASE REVIEW CAREFULLY SECTION
            XIII “DISPUTE RESOLUTION” BELOW FOR DETAILS REGARDING ARBITRATION.
          </p>

          <h2 className="text-2xl font-bold mb-4 mt-10 text-left">
            1. Eligibility
          </h2>
          <p className="text-base leading-relaxed mb-16 text-left">
            The Contest is open to all individuals who have reached the age of
            majority in their jurisdiction. Participants must have a compatible
            crypto wallet and access to the SUI blockchain. Employees of the
            organizers and their immediate families are not eligible to win.
          </p>
          <h2 className="text-2xl font-bold mb-4 mt-10 text-left">
            2. Contest Overview
          </h2>
          <p className="text-base leading-relaxed mb-16 text-left">
            The Contest is designed to showcase creativity using the Walrus
            decentralized storage protocol. Participants will mint their memes
            as NFTs, which will be available for public voting. The winning NFT
            will be determined by the highest number of votes.
          </p>
          <h2 className="text-2xl font-bold mb-4 mt-10 text-left">
            3. Submission Guidelines
          </h2>
          <p className="text-base leading-relaxed mb-16 text-left">
            - All submissions must adhere to the theme of the contest and be
            original works created by the participant.
            <br />
            - Content that violates any intellectual property rights or contains
            offensive, discriminatory, or inappropriate material will be
            disqualified.
            <br />
            - Each participant is responsible for ensuring their submission
            complies with local laws and contest rules.
            <br />
          </p>
          <h2 className="text-2xl font-bold mb-4 mt-10 text-left">
            4. Voting and Winner Selection
          </h2>
          <p className="text-base leading-relaxed mb-16 text-left">
            Votes will be cast by the public, with each NFT available for
            review. The NFT with the highest number of valid votes at the end of
            the contest period will be declared the winner. The organizers
            reserve the right to disqualify any votes they deem fraudulent.
          </p>
          <h2 className="text-2xl font-bold mb-4 mt-10 text-left">5. Prizes</h2>
          <p className="text-base leading-relaxed mb-16 text-left">
            The winning participant will receive [details of the prize] and
            recognition within the Walrus community. Prizes are non-transferable
            and cannot be exchanged for cash or other rewards.
          </p>
          <h2 className="text-2xl font-bold mb-4 mt-10 text-left">
            6. Intellectual Property
          </h2>
          <p className="text-base leading-relaxed mb-16 text-left">
            By submitting an NFT, participants grant the organizers a
            non-exclusive, royalty-free license to use, display, and promote the
            NFT and associated meme for marketing purposes related to the
            Contest.
          </p>
          <h2 className="text-2xl font-bold mb-4 mt-10 text-left">
            7.  Reporting and Disqualification
          </h2>
          <p className="text-base leading-relaxed mb-16 text-left">
            Any participant found violating these terms or submitting
            inappropriate content will be disqualified. Submissions can be
            reported for reasons including but not limited to:
            <br />
            - Inappropriate content
            <br />
            - Copyright infringement
            <br />- Misleading or fraudulent information
          </p>
          <h2 className="text-2xl font-bold mb-4 mt-10 text-left">
            8.  Limitation of Liability
          </h2>
          <p className="text-base leading-relaxed mb-16 text-left">
            The organizers are not responsible for any issues arising from
            blockchain technology, wallet failures, or third-party actions.
            Participation is at the participant’s own risk.
            <br />
            <br />
            The organizers reserve the right to modify these Terms and
            Conditions at any time. Changes will be effective immediately upon
            posting on this page. By participating, you agree to these Terms and
            Conditions. For any inquiries, please contact us at [email/contact
            information].
            <br />
            <br />
            This draft aligns with standard Web3 contest practices and includes
            relevant legal disclaimers for decentralized platforms. Feel free to
            modify details like the prize and any specific rules.
            <br />
            <br />
            If you have any questions about these Terms or the Services, please
            contact Sui at legal@mystenlabs.com.
          </p>
        </div>
        <Footer />
      </div>
    </div>
  );
};

export default ToSPage;
