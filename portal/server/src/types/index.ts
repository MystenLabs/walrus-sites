// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { StorageVariant } from "../enums";
import BlocklistChecker from "@lib/blocklist_checker";

export type ListChecker = BlocklistChecker; // TODO AllowlistChecker
export type CheckerMap = {
    [key in StorageVariant]: () => ListChecker;
}
