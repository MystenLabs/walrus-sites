# Walrus Sites!

This repo contains the various components of a working [Walrus
Site](https://docs.walrus.site/walrus-sites/intro.html). The resources of a Walrus Site are stored
on Walrus, and the metadata and ownership of the site is managed through Sui.

Documentation of Walrus and Walrus Sites is available at [docs.walrus.site](https://docs.walrus.site).

## Repo structure

- [`move`](./move/) contains the smart contract for creating and updating Walrus Sites on chain.
- [`portal`](./portal/) is the implementation of a portal to access Walrus Sites. It is based on
  service workers.
- The [`site-builder`](./site-builder/) is a Rust cli tool to create Walrus Sites, starting from the
  output of a standard website building tool (i.e., a directory containing html/css/js).
- In [`examples`](./examples/) there is a collection of websites to test the functions of the Walrus
  Sites.
