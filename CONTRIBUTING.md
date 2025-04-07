# How to Contribute

## Important note

> [!IMPORTANT]
> We highly appreciate contributions, but **simple typo fixes (e.g., minor spelling errors,
> punctuation changes, or trivial rewording) will be ignored** unless they significantly improve
> clarity or fix a critical issue. If you are unsure whether your change is substantial enough,
> consider opening an issue first to discuss it.

## GitHub flow

We generally follow the [GitHub flow](https://docs.github.com/en/get-started/quickstart/github-flow) in our project. In
a nutshell, this requires the following steps to contribute:

1. [Fork the repository](https://docs.github.com/en/get-started/quickstart/contributing-to-projects) (only required if
   you don't have write access to the repository).
1. [Create a feature branch](https://docs.github.com/en/get-started/quickstart/github-flow#create-a-branch).
1. [Make changes and create a
   commit](https://docs.github.com/en/get-started/quickstart/contributing-to-projects#making-and-pushing-changes)
1. Push your changes to GitHub and [create a pull
   request](https://docs.github.com/en/get-started/quickstart/contributing-to-projects#making-a-pull-request) (PR);
   note that we enforce a particular style for the PR titles, see [below](#commit-messages).
1. Wait for maintainers to review your changes and, if necessary, revise your PR.
1. When all requirements are met, a reviewer or the PR author (if they have write permissions) can merge the PR.

## Commit messages

To ensure a consistent Git history (from which we can later easily generate changelogs
automatically), we always squash commits when merging a PR and enforce that all PR titles comply
with the [conventional-commit format](https://www.conventionalcommits.org/en/v1.0.0/). For examples,
please take a look at our [commit history](https://github.com/MystenLabs/walrus/commits/main).

## Documentation

Make sure all public structs, enums, functions, etc. are covered by docstrings. Additionally, if you
made any user-facing changes, please adjust our documentation under
[docs/book](https://github.com/MystenLabs/walrus/tree/main/docs).

## Pre-commit hooks

We have CI jobs running for every PR to test and lint the repository. You can install Git pre-commit
hooks to ensure that these check pass even *before pushing your changes* to GitHub. To use this, the
following steps are required:

1. Install [Rust](https://www.rust-lang.org/tools/install).
1. [Install pre-commit](https://pre-commit.com/#install) using `pip` or your OS's package manager.
1. Run `pre-commit install` in the repository.

After this setup, the code will be checked, reformatted, and tested whenever you create a Git commit.

You can also use a custom pre-commit configuration if you wish:

1. Create a file `.custom-pre-commit-config.yaml` (this is set to be ignored by Git).
1. Run `pre-commit install -c .custom-pre-commit-config.yaml`.

## Formatting

### Rust code formatting

We use a few unstable formatting options of Rustfmt. Unfortunately, these can only be used with a
stable toolchain when specified via the `--config` command-line option. This is done in
[CI](.github/workflows/code.yml) and in our [pre-commit hooks](.pre-commit-config.yaml) (see also
[above](#pre-commit-hooks)).

If you want the same behavior in your IDE, you need to modify the corresponding formatting setting.
For example, when using `rust-analyzer` with VSCode, you need to add the following to your
`settings.json`:

```json
    "rust-analyzer.rustfmt.extraArgs": [
        "--config",
        "group_imports=StdExternalCrate,imports_granularity=Crate,imports_layout=HorizontalVertical"
    ]
```

Also make sure you use the correct version of Rustfmt. See
[`rust-toolchain.toml`](rust-toolchain.toml) for the current version. This also impacts other checks,
for example Clippy.

### Move code formatting

We use the `@mysten/prettier-plugin-move` npm package to format Move code. If you're using VSCode,
you can install the [Move Formatter](https://marketplace.visualstudio.com/items?itemName=mysten.prettier-move)
extension. The formatter is also run automatically in the [pre-commit hooks](#pre-commit-hooks).

To use it as a stand-alone tool, we recommend installing it globally (requires NodeJS and npm):

```sh
npm i -g prettier @mysten/prettier-plugin-move
```

The Move formatter can then be run manually by executing:

```sh
prettier-move --write <path-to-move-file-or-folder>
```

## Tests

The majority of our code is covered by automatic unit and integration tests which you can run
through `cargo test`.

Integration and end-to-end tests are excluded by default when running `cargo test` as they depend on
additional packages and take longer to run. You can run these test as follows:

```sh
cargo test -- --ignored
```

## Signed commits

We appreciate it if you configure Git to [sign your
commits](https://gist.github.com/troyfontaine/18c9146295168ee9ca2b30c00bd1b41e).
