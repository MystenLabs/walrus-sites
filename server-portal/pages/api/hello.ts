// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

export const config = {
  runtime: 'edge'
}

export default function handler(req: Request) {
  return new Response("Hello World");
}

