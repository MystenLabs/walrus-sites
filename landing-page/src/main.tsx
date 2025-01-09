// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "./index.css";
import App from "./App";
import { SuiClientProvider, WalletProvider } from "@mysten/dapp-kit";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { networkConfig } from "./networkConfig";
import {
  ENOKI_PUBLIC_KEY,
  NETWORK_NAME,
  RECAPTCHA_PUBLIC_KEY,
} from "./config/globalVariables";
import { EnokiFlowProvider } from "@mysten/enoki/react";
import { GoogleReCaptchaProvider } from "react-google-recaptcha-v3";

const queryClient = new QueryClient();

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <EnokiFlowProvider apiKey={ENOKI_PUBLIC_KEY}>
      <GoogleReCaptchaProvider reCaptchaKey={RECAPTCHA_PUBLIC_KEY}>
        <QueryClientProvider client={queryClient}>
          <SuiClientProvider
            networks={networkConfig}
            defaultNetwork={NETWORK_NAME}
          >
            <WalletProvider
              autoConnect
              stashedWallet={{
                name: "Walrus Adventure",
              }}
            >
              <App />
            </WalletProvider>
          </SuiClientProvider>
        </QueryClientProvider>
      </GoogleReCaptchaProvider>
    </EnokiFlowProvider>
  </StrictMode>
);
