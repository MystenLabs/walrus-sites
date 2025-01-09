// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import React, { useEffect, useState } from "react";
import { ConnectButton } from "@mysten/dapp-kit";
import { Link } from "react-router-dom";

const MinimizedHeader: React.FC = () => {
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
    <header className="sticky top-0 z-50 bg-gradient-to-t from-[#0C0F1D80] to-[#97F0E580] py-4 px-2">
      <div className="mx-auto flex justify-between items-center bg-primary_dark rounded-xl py-3 px-6">
        <Link to="/">
          <img
            src="/logo.png"
            alt="Walrus Adventure"
            className="custom_lg:w-[280px] md:w-[220px] w-[160px] h-auto"
          />
        </Link>
        <ConnectButton connectText="CONNECT WALLET" style={buttonStyle} />
      </div>
    </header>
  );
};

export default MinimizedHeader;
