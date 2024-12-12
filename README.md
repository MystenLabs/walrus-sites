# Walrus Sites!

This repo contains the various components of a working [Walrus
Site](https://docs.walrus.site/walrus-sites/intro.html). The resources of a Walrus Site are stored
on Walrus, and the metadata and ownership of the site is managed through Sui.

Documentation of Walrus and Walrus Sites is available at [docs.walrus.site](https://docs.walrus.site/walrus-sites/intro.html).

## Repo structure

- [`move`](./move/) contains the smart contract for creating and updating Walrus Sites on chain.
- [`portal`](./portal/) contains the implementations of the portals to access Walrus Sites.
- The [`site-builder`](./site-builder/) is a Rust cli tool to create Walrus Sites, starting from the
  output of a standard website building tool (i.e., a directory containing html/css/js).
- In [`examples`](./examples/) there is a handy collection of websites to test the functions of the Walrus
  Sites.
- In [`c4`](./c4/) there is the C4 model depicting the architecture of the project.

## Star History

Walrus Sites is open source! Here is a graph showing the star history over time.

[![Star History Chart](https://api.star-history.com/svg?repos=MystenLabs/walrus-sites&type=Date)](https://star-history.com/#MystenLabs/walrus-sites&Date)
