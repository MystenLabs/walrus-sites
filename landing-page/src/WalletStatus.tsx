// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { useCurrentAccount } from "@mysten/dapp-kit";

export function WalletStatus() {
  const account = useCurrentAccount();

  return (
    <div className="container mx-auto my-4 p-4">
      <h2 className="text-2xl font-bold mb-4 text-white">Wallet Status</h2>

      {account ? (
        <div className="flex flex-col space-y-2">
          <p className="text-white">Wallet connected</p>
          <p className="text-white">Address: {account.address}</p>
        </div>
      ) : (
        <p className="text-white">Wallet not connected</p>
      )}
    </div>
  );
}
