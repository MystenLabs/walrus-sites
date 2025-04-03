# Walrus Sites!

[![License](https://img.shields.io/github/license/MystenLabs/walrus-sites)](https://github.com/MystenLabs/walrus-sites/blob/main/LICENSE)

Walrus Sites are "web"-sites built using decentralized tech such as [Walrus][walrus-link]
a decentralized storage network, and the [Sui blockchain][sui-link].

This means that there is no central authority hosting the sites. Only the owner of the site is in
charge of the site's content and updates.

[Use cases][use-cases] can span from censorship resistant sites to decentralized applications.

## Overview

At a high level, Walrus is used to store the files of the site (your CSS, JS, HTML etc.),
while a Sui smart contract is used to manage the metadata and ownership of the site.

Once you publish a site using the  `site-builder` CLI tool, you can access a site using a `portal`.
Portals are centralized services that provide a way to gather the site resources and serve them to the user.

> Anyone can host their own portal and access Walrus Sites freely!

To browse walrus sites, you can either host your own local portal or access a third-party deployment
on the internet like https://wal.app.

Domain resolution of Walrus Sites is done with a combination of traditional wildcard domains (e.g. `*.wal.app`)
and the use of subdomains that correspond to [SuiNS domains][suisns].

For example, let's analyze the following site: https://stake-wal.wal.app

1. We are provided with a third-party portal: https://wal.app.
2. The site SuiNS domain is `stake`, so we need to use it as a subdomain of wal.app: `stake-wal.wal.app`.
3. Therefore, we can access the site with standard HTTPS at `https://stake-wal.wal.app`.

Documentation of Walrus and Walrus Sites is available at [docs.wal.app][walrus-sites-docs].

> Fun fact: the documentation is itself a Walrus Site!

## Quick Start

Walrus Sites undergo a lot of changes, so the best way to avoid confusion with deprecated features
please start by following the guide [here](https://docs.wal.app/walrus-sites/intro.html).

## File structure

This repository contains the various components of a working Walrus Site.
To navigate the repository, these are the crucial directories you should know about:

- The [`site-builder`](./site-builder/) is a Rust CLI tool to create and edit your Walrus Sites, starting from the
  output of a standard website building tool (i.e., a directory containing html/css/js, like `dist/` or `build/`).
- [`portal`](./portal/) contains the implementations of the portals to access Walrus Sites.
  - [`server`](./portal/server/) contains the implementation of an HTTP server portal. When in doubt, deploy this one.
  - [`worker`](./portal/worker/) contains the implementation of a [service-worker][service-worker] portal.
- In [`examples`](./examples/) there is a handy collection of websites to test the functions of the Walrus
  Sites.
- [`move`](./move/) contains the smart contract for creating and updating Walrus Sites on chain.
- In [`c4`](./c4/) there is the C4 model depicting the architecture of the project.

## Star History

Walrus Sites is open source! Here is a graph showing the star history over time.

[![Star History Chart](https://api.star-history.com/svg?repos=MystenLabs/walrus-sites&type=Date)](https://star-history.com/#MystenLabs/walrus-sites&Date)

[walrus-link]: https://www.walrus.xyz/
[sui-link]: https://docs.sui.io/
[use-cases]: https://docs.wal.app/design/objectives_use_cases.html#use-cases
[walrus-sites-docs]: https://docs.wal.app/walrus-sites/intro.html
[service-worker]: https://developer.mozilla.org/en-US/docs/Web/API/Service_Worker_API
[suins]: https://suins.io/
