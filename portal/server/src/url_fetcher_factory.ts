// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { UrlFetcher } from "@lib/url_fetcher";
import { ResourceFetcher } from "@lib/resource";
import { RPCSelector } from "@lib/rpc_selector";
import { SuiNSResolver } from "@lib/suins";
import { WalrusSitesRouter } from "@lib/routing";
import { PriorityExecutor } from "@lib/priority_executor";
import { config } from "./configuration_loader";

const rpcSelector = new RPCSelector(config.rpcUrlList, config.suinsClientNetwork);
const aggregatorExecutor = new PriorityExecutor(config.aggregatorUrlList);

export const urlFetcher = new UrlFetcher(
    new ResourceFetcher(rpcSelector, config.sitePackage),
    new SuiNSResolver(rpcSelector),
    new WalrusSitesRouter(rpcSelector),
    aggregatorExecutor,
    config.b36DomainResolutionSupport,
);
