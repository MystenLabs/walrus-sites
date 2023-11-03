import { getFullnodeUrl } from "@mysten/sui.js/client";
import {
  LOCALNET_PACKAGE_ID,
  DEVNET_PACKAGE_ID,
  MAINNET_PACKAGE_ID,
} from "./constants.ts";
import { createNetworkConfig } from "@mysten/dapp-kit";

const { networkConfig, useNetworkVariable, useNetworkVariables } =
  createNetworkConfig({
    localnet : {
      url: getFullnodeUrl("localnet"),
      variables: {
        blocksitePackageId: LOCALNET_PACKAGE_ID,
      },
    },
    devnet: {
      url: getFullnodeUrl("devnet"),
      variables: {
        blocksitePackageId: DEVNET_PACKAGE_ID,
      },
    },
    mainnet: {
      url: getFullnodeUrl("mainnet"),
      variables: {
        blocksitePackageId: MAINNET_PACKAGE_ID,
      },
    },
  });

export { useNetworkVariable, useNetworkVariables, networkConfig };
