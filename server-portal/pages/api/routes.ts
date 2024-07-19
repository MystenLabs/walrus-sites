// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0


export const config = {
    runtime: 'edge',
}

export default async function handler(request: Request) {
  const url = new URL(request.url);
}
