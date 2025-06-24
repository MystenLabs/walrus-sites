// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use clap::Parser;
use site_builder::args::Args;

// Define the `GIT_REVISION` and `VERSION` consts.
bin_version::bin_version!();

#[derive(Parser, Debug)]
#[command(rename_all = "kebab-case", version = VERSION, propagate_version = true)]
struct App {
    #[command(flatten)]
    inner: Args,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    tracing::info!("initializing site builder");

    let args = App::parse();
    tracing::debug!(?args, "command line arguments");
    let App { inner } = args;
    site_builder::run(inner).await
}
