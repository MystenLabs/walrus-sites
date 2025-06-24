// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use clap::Parser;
use site_builder::args::ArgsInner;

// Define the `GIT_REVISION` and `VERSION` consts.
bin_version::bin_version!();

#[derive(Parser, Debug)]
#[command(rename_all = "kebab-case", version = VERSION, propagate_version = true)]
struct Args {
    #[command(flatten)]
    inner: ArgsInner,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    tracing::info!("initializing site builder");

    let args = Args::parse();
    tracing::debug!(?args, "command line arguments");
    let Args { inner } = args;
    site_builder::run(inner).await
}
