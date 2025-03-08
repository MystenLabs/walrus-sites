// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { UrlFetcher } from "@lib/url_fetcher";
import { ResourceFetcher } from "@lib/resource";
import { RPCSelector } from "@lib/rpc_selector";
import { SuiNSResolver } from "@lib/suins";
import { WalrusSitesRouter } from "@lib/routing";
import { config } from "./configuration_loader";

/**
* A factory class for creating page fetchers.
* Page fetchers can be either premium or standard.
* Premium fetchers use premium RPC nodes that can serve content faster and more reliably,
* while standard fetchers use standard RPC nodes.
*/
class UrlFetcherFactory {
    private static readonly premiumRpcSelector = new RPCSelector(
        config.premiumRpcUrlList, config.suinsClientNetwork
    );
    private static readonly standardRpcSelector = new RPCSelector(
        config.rpcUrlList, config.suinsClientNetwork
    );

    public static premiumUrlFetcher(): UrlFetcher {
        return new UrlFetcher(
            new ResourceFetcher(this.premiumRpcSelector, config.sitePackage),
            new SuiNSResolver(this.premiumRpcSelector),
            new WalrusSitesRouter(this.premiumRpcSelector),
            config.aggregatorUrl,
            config.b36DomainResolutionSupport
        );
    }

    public static standardUrlFetcher(): UrlFetcher {
        return new UrlFetcher(
            new ResourceFetcher(this.standardRpcSelector, config.sitePackage),
            new SuiNSResolver(this.standardRpcSelector),
            new WalrusSitesRouter(this.standardRpcSelector),
            config.aggregatorUrl,
            config.b36DomainResolutionSupport
        );
    }
}

export const standardUrlFetcher = UrlFetcherFactory.standardUrlFetcher();
export const premiumUrlFetcher = UrlFetcherFactory.premiumUrlFetcher();
