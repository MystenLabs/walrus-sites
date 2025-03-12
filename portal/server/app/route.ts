// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0
import main from "src/main";
import { NextRequest } from "next/server";

export async function GET(req: NextRequest) {
	return main(req)
}
