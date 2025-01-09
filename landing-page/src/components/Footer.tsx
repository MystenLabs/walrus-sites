// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import React from "react";

const Footer: React.FC = () => {
  return (
    <>
      {/* Mobile view footer */}
      <footer className="w-full bg-dark-800 text-gray-400 py-5 flex flex-col items-center space-y-6 md:hidden">
        <div className="flex items-center space-x-8">
          <a
            href="https://discord.com/invite/walrusprotocol"
            aria-label="Discord"
            target="_blank"
            rel="noopener noreferrer"
          >
            <img
              src="/discord_logo.png"
              alt="Discord logo"
              className="w-5 h-5 hover:opacity-80"
            />
          </a>

          <a
            href="https://x.com/WalrusProtocol"
            aria-label="X"
            target="_blank"
            rel="noopener noreferrer"
          >
            <img
              src="/x_logo.png"
              alt="X logo"
              className="w-4 h-4 hover:opacity-80"
            />
          </a>
        </div>
        <div>
          <a
            href="/terms-of-service"
            className="text-sm font-bold text-white hover:text-gray-200 pr-4"
          >
            Terms of Service
          </a>
          <a
            href="/privacy-policy"
            className="text-sm font-bold text-white hover:text-gray-200 pl-4"
          >
            Privacy Policy
          </a>
        </div>

        <div className="text-sm text-[#6E609F] text-center">
          Copyright {new Date().getFullYear()} © Mysten Labs, Inc.
        </div>
      </footer>

      {/* Desktop view footer */}
      <footer className="w-full bg-dark-800 text-gray-400 py-10   justify-between items-center max-w-[1100px] hidden md:flex">
        <div className="text-sm text-[#6E609F]">
          Copyright {new Date().getFullYear()} © Mysten Labs, Inc.
        </div>

        <div className="flex items-center space-x-8">
          <a
            href="https://discord.com/invite/walrusprotocol"
            aria-label="Discord"
            target="_blank"
            rel="noopener noreferrer"
          >
            <img
              src="/discord_logo.png"
              alt="Discord logo"
              className="w-5 h-5 hover:opacity-80"
            />
          </a>

          <a
            href="https://x.com/WalrusProtocol"
            aria-label="X"
            target="_blank"
            rel="noopener noreferrer"
          >
            <img
              src="/x_logo.png"
              alt="X logo"
              className="w-4 h-4 hover:opacity-80"
            />
          </a>

          {/* <a
            href="/faq"
            className="text-sm font-bold text-white hover:text-gray-200"
          >
            FAQ
          </a> */}
          <a
            href="/terms-of-service"
            className="text-sm font-bold text-white hover:text-gray-200"
          >
            Terms of Service
          </a>
          <a
            href="/privacy-policy"
            className="text-sm font-bold text-white hover:text-gray-200"
          >
            Privacy Policy
          </a>
        </div>
      </footer>
    </>
  );
};

export default Footer;
