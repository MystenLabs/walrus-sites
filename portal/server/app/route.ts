// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0
import main from "src/main";

export async function GET(req: Request) {
	return main(req)
}
