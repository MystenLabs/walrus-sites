# Examples of Blocksites

This directory contains two examples of Blocksites:
- a multipage landing website (`./landing-blocksite/`); and
- an interactive chat DApp, blockchat (`./blockchat/`).

## Prerequisites

These examples can be run on testnet (or mainnet). Running them has several
prerequisites, both on other code within this repository, as well as the host
system:

- The `sui` [client CLI](https://docs.sui.io/references/cli/client) should be
  installed and available on path.

- `pnpm` should be installed and available on path.

- The site builder should already be built:
    ```rust
    cargo build --release --manifest-path ../site-builder/Cargo.toml
    ```
  and a config should be created, see `../site-builder/config-example.toml` for
  an example of a config.

- The blocksite move module should already be published to the chain. On 
  testnet, this is already published at package-id 

  ```text
  0x66b0b2d46dcd2e56952f1bd9e90218deaab0885e0f60ca29163f5e53c72ef810
  ```
  and is the default in the config. Alternatively you can publish your own 
  instance, for example on mainnet or devnet, from the smart contracts in
  `../move/blocksite`.


## Examples

This section describes how to publish the example blocksites found in this 
directory.

#### Viewing the Published Blocksites

On publishing the various examples to Sui, the `site-builder` utility will provide
you with URLs from which you can load the live blocksite. See `../portal/` for
instructions on running your own local portal to view your live publised website.

### Example: Landing-Blocksite

This blocksite is an multipage landing site with a functioning version of Snake
(the game) on a subpage, as well as some general information on blocksites. It
is HTML, javascript, and CSS and does not require an additional smart contract.

#### Publishing the Blocksites

As this site is already is purely HTML and its associated resources, it can be
directly published using the `site-builder` utility:

```shell
../site-builder/target/release/site-builder --config config.toml publish \
    landing-blocksite/ --content-encoding gzip
```

### Example: Blockchat 

This is an example blocksite for a chat DApp. It consists of both a smart
contract that stores and transmits the messages, as well as the Blocksite that
interacts with the smart contract to provide the frontend. These components
can be found in `blockchat/move/blockchat` and `blockchat/dapp` respectively.

#### Publishing the Smart Contract

The smart contract for blockchat can be published like any other smart contract
on Sui, using `sui client`:

```shell
sui client publish --gas-budget 20000000 blockchat/move/blockchat
```

and then create a new chat manually using Sui, with the package ID you obtained
in the previous step as `<package-id>`:

```shell
sui client call --function create_chat --module blockchat \
    --package <package-id> --gas-budget 10000000 --args "My Chat"
```

Next, update the `PACKAGE_ID`, `CHAT_ID`, and optionally, `NETWORK` in 
`blockchat/dapp/src/Messages.tsx` to match those that you created on chain, 
then proceed below to publish the blocksite.

#### Publishing the Blocksite

This blocksite is built with `pnpm` and `typescript`. To publish the blocksite,
we will first need to install any dependencies and build it with `pnpm`. These
steps are identical to building any other site with `pnpm`:

```shell
pnpm --dir blockchat/dapp install
pnpm --dir blockchat/dapp run build
```

Next, we must publish the resulting index page and assets to objects on Sui,
from where they will be served. We do this using the `site-builder` utility
from this repository:

```shell
../site-builder/target/release/site-builder --config config.toml publish \
    blockchat/dapp/dist --content-encoding gzip
```
