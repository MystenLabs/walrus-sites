// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::path::PathBuf;

use clap::Parser;
use site_builder::args::{Commands, GeneralArgs};

// Define the `GIT_REVISION` and `VERSION` consts.
bin_version::bin_version!();

#[derive(Parser, Debug)]
#[command(rename_all = "kebab-case", version = VERSION, propagate_version = true)]
struct Args {
    /// The path to the configuration file for the site builder.
    #[arg(short, long)]
    config: Option<PathBuf>,
    /// The context with which to load the configuration.
    ///
    /// If specified, the context will be taken from the config file. Otherwise, the default
    /// context, which is also specified in the config file, will be used.
    #[arg(long)]
    context: Option<String>,
    #[clap(flatten)]
    general: GeneralArgs,
    #[command(subcommand)]
    command: Commands,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    tracing::info!("initializing site builder");

    let args = Args::parse();
    tracing::debug!(?args, "command line arguments");
    let Args {
        config,
        context,
        general,
        command,
    } = args;
    site_builder::run(config, context, general, command).await
}
