// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { UrlFetcher } from "@lib/url_fetcher";
import { ResourceFetcher } from "@lib/resource";
import { RPCSelector } from "@lib/rpc_selector";
import { SuiNSResolver } from "@lib/suins";
import { WalrusSitesRouter } from "@lib/routing";
import { config } from "configuration_loader";

/**
* A factory class for creating page fetchers.
* Page fetchers can be either premium or standard.
* Premium fetchers use premium RPC nodes that can serve content faster and more reliably,
* while standard fetchers use standard RPC nodes.
*/
class UrlFetcherFactory {
    private static readonly premiumRpcSelector = new RPCSelector(config.premiumRpcUrlList);
    private static readonly standardRpcSelector = new RPCSelector(config.rpcUrlList);

    public static premiumUrlFetcher(): UrlFetcher {
        return new UrlFetcher(
            new ResourceFetcher(this.standardRpcSelector),
            new SuiNSResolver(this.standardRpcSelector),
            new WalrusSitesRouter(this.standardRpcSelector)
        );
    }

    public static standardUrlFetcher(): UrlFetcher {
        return new UrlFetcher(
            new ResourceFetcher(this.premiumRpcSelector),
            new SuiNSResolver(this.premiumRpcSelector),
            new WalrusSitesRouter(this.premiumRpcSelector)
        );
    }
}

export const standardUrlFetcher = UrlFetcherFactory.standardUrlFetcher();
export const premiumUrlFetcher = UrlFetcherFactory.premiumUrlFetcher();
