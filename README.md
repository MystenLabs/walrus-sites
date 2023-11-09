# Blocksites!

This repo contains the various components for a working blocksite proof of concept.
A "blocksite" is a website that is hosted in owned objects on sui.

## How does it work?

See the [description](https://blocksite-portal-poc.vercel.app/#0x6e95fa8fff2147583f42d54ed4352505e8556b6fd5e27a75f354cee910182bc8) and [features](https://blocksite-portal-poc.vercel.app/#0x491effc375c2cb94fbb9459fb3185b5c0800c52c9252d2d045f6dfad89fb8487) on the respective blocksites!

## Repo structure

- `portal/` contains the code for the portal dApp, which is the entry point to the blocksite "web". 
Through the portal, users can browse blocksites and create new ones.
- `move/` constains the smart contract for creating and updating blocksites on chain.
- In `test-sites/` there is a collection of one-page websites describing the project that are used as tests. These sites are inlined with `inliner` (`$ npm install -g inliner`)

## Limitations and known issues

### THE WALLET DOES NOT CONNECT

This is the main limitation at the moment. For good reasons, the wallet does not connect to content loaded in `iframe`s, which is where the blocksites are loaded.

The only viable solution to this problem is to modify the blocksite portal to expose a "proxy" interface through which the inner page can send transactions, which are then relayed to the wallet. Although this is not desirable in general, in this particular case the user needs to trust the portal in any case for loading the blocksite, and therefore the "proxy" does not add further risks.

I have no idea how to implement such a proxy; PRs welcome.

### Cost and blocksite size

The object size limits impose a few limitations on the blocksites: 
- The biggest single-file blocksite that can be created through the portal is `~130KB` _compressed_. This is because of the limitation on transaction sizes
- The biggest blocksite that can now be created (through `cli` calls) is `~250KB` compressed, because of the maximum object size. This can be increased by eventually spreading a blocksite over multiple objects.
- `js/css` sources have to be inlined at the moment, as there is currently no way to have another object as `href` for raw content.
- Updating is basically the same as creating a new blocksite. 

### Security issues

TBD! (blocksites are loaded in a sandboxed `iframe`, with many `allow-*`s)
