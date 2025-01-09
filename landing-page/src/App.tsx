// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import React, { useEffect } from "react";
import { BrowserRouter as Router, Routes, Route } from "react-router-dom";
import HomePage from "./pages/HomePage";
import ToSPage from "./pages/ToSPage";
import PrivacyPolicyPage from "./pages/PrivacyPolicyPage";
import { ToastContainer } from "react-toastify";
import "react-toastify/dist/ReactToastify.css";
import Custom404 from "./pages/Custom404";
import * as amplitude from "@amplitude/analytics-browser";
import { AMPLITUDE_API_KEY } from "./config/globalVariables";

const App: React.FC = () => {
  useEffect(() => {
    try {
      amplitude.init(AMPLITUDE_API_KEY, {
        autocapture: true,
        identityStorage: "none",
      });
    } catch (e) {
      console.error("Amplitude initialization failed:", e);
    }
  }, []);
  return (
    <Router>
      <ToastContainer
        position="top-right"
        autoClose={5000}
        hideProgressBar={false}
        newestOnTop={false}
        closeOnClick
        rtl={false}
        pauseOnFocusLoss
        draggable
        pauseOnHover
        theme="dark"
      />
      <Routes>
        <Route path="/" element={<HomePage />} />
        <Route path="/meme-details/:id" element={<HomePage />} />
        <Route path="/terms-of-service" element={<ToSPage />} />
        <Route path="/privacy-policy" element={<PrivacyPolicyPage />} />
        <Route path="*" element={<Custom404 />} />
      </Routes>
    </Router>
  );
};

export default App;
