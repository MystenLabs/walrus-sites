# Walrus sites!

This repo contains the various components of a working Walrus site.
The resources of a Walrus site are stored on Walrus, and the metadata and ownership of the site is managed through Sui.

## Repo structure

- `move` contains the smart contract for creating and updating Walrus sites on chain.
- `portal` is the implementation of a portal to access Walrus sites. It is based on service workers.
- The `site-builder` is a Rust cli tool to create Walrus sites, starting from the output of a standard website building tool (i.e., a directory
  containing html/css/js).
- In `examples` there is a collection of websites to test the functions of the Walrus sites.
