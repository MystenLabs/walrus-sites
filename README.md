# Blocksites!

This repo contains the various components for a working blocksite proof of concept.
A "blocksite" is a website that is hosted in owned objects on sui.

## How does it work?

See the [description](https://blocksite-portal-poc.vercel.app/#0x93e9e43be38372f7915aac3bdd30e5c2f2d22c699475e5944f06d8fb67b6874c) and [features](https://blocksite-portal-poc.vercel.app/#0x9bfd168a1f3efe281dab315a552249c6e08b01d89fda7cd8b89a28bb68b7d644) on the respective blocksites!

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

## Blocksite addresses on devnet

- `image-blocksite`: `0x83eb6879ed4ef76cced88c70c806b982e315afc2ba23b8afd189ff098aec4080`
- `snake-blocksite`: `0x50182427c57fb6c050bcbb3755bde94a99749da4507242f2402d31549a2fd12d`
- `features-blocksite`: `0x9bfd168a1f3efe281dab315a552249c6e08b01d89fda7cd8b89a28bb68b7d644`
- `landing-blocksite`: `0x93e9e43be38372f7915aac3bdd30e5c2f2d22c699475e5944f06d8fb67b6874c`