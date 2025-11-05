# ts-sdk-site-builder

An attempt to create a typescript version of the site-builder.

The smart contract integration is done automatically through the [sui-codegen](https://www.npmjs.com/package/@mysten/codegen) tool.

## Project structure

```
ts-sdk-site-builder/
├── cli/                 # CLI interface, argument parsing
├── flows/               # Orchestration of contract calls
│   └── publish.ts       # Publish site flow
└── contracts/
    └── sites/           # Low-level contract interactions
```
