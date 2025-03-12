// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0
import main from "src/main";
import { Request } from "next/server";

export async function GET(req: Request) {
	return main(req)
}
