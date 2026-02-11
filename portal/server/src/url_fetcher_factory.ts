// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { UrlFetcher } from "@lib/url_fetcher";
import { ResourceFetcher } from "@lib/resource";
import { RPCSelector } from "@lib/rpc_selector";
import { SuiNSResolver } from "@lib/suins";
import { WalrusSitesRouter } from "@lib/routing";
import { PriorityExecutor } from "@lib/priority_executor";
import { config } from "./configuration_loader";

/**
 * A factory class for creating page fetchers.
 * Page fetchers can be either premium or standard.
 * Premium fetchers use premium RPC nodes that can serve content faster and more reliably,
 * while standard fetchers use standard RPC nodes.
 */
class UrlFetcherFactory {
    private static readonly premiumRpcSelector = config.premiumRpcUrlList
        ? new RPCSelector(config.premiumRpcUrlList, config.suinsClientNetwork)
        : undefined;
    private static readonly standardRpcSelector = new RPCSelector(
        config.rpcUrlList,
        config.suinsClientNetwork,
    );

    private static readonly aggregatorExecutor = new PriorityExecutor(config.aggregatorUrlList);

    public static premiumUrlFetcher(): UrlFetcher | undefined {
        if (!this.premiumRpcSelector) return undefined;
        return new UrlFetcher(
            new ResourceFetcher(this.premiumRpcSelector, config.sitePackage),
            new SuiNSResolver(this.premiumRpcSelector),
            new WalrusSitesRouter(this.premiumRpcSelector),
            this.aggregatorExecutor,
            config.b36DomainResolutionSupport,
        );
    }

    public static standardUrlFetcher(): UrlFetcher {
        return new UrlFetcher(
            new ResourceFetcher(this.standardRpcSelector, config.sitePackage),
            new SuiNSResolver(this.standardRpcSelector),
            new WalrusSitesRouter(this.standardRpcSelector),
            this.aggregatorExecutor,
            config.b36DomainResolutionSupport,
        );
    }
}

export const standardUrlFetcher = UrlFetcherFactory.standardUrlFetcher();
export const premiumUrlFetcher = UrlFetcherFactory.premiumUrlFetcher();
