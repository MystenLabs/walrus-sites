// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import React from "react";
import { useNavigate } from "react-router-dom";

const Custom404: React.FC = () => {
  const navigate = useNavigate();

  const handleGoBack = () => {
    navigate("/");
  };

  return (
    <div className="flex items-center justify-center min-h-screen bg-primary_dark text-white font-ppNeueMontreal">
      <div className="text-center">
        <h1 className="text-8xl font-extrabold bg-gradient-to-r from-[#FF5C5C] to-[#FFC542] bg-clip-text text-transparent">
          404
        </h1>
        <p className="mt-4 text-lg text-gray-400">
          Oops! The page you’re looking for doesn’t exist.
        </p>
        <div className="mt-6">
          <button
            onClick={handleGoBack}
            className="bg-primary_teal text-black py-2 px-6 rounded-lg hover:bg-[#00FFC2] font-semibold"
          >
            Go Back to Home
          </button>
        </div>
      </div>
    </div>
  );
};

export default Custom404;
