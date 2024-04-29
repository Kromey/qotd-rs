use std::{fs::File, sync::Mutex};

use anyhow::Context;
use clap::Parser;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, Layer};

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    let args = qotd::Cli::parse();

    // Set up our logging
    let registry = tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().with_filter(args.verbosity()));
    if let Some(log_path) = &args.log_file {
        let log_file = File::create(log_path).context("Unable to create log file")?;
        registry
            .with(
                tracing_subscriber::fmt::layer()
                    .with_ansi(false)
                    .with_writer(Mutex::new(log_file))
                    .with_filter(args.file_verbosity()),
            )
            .init();
    } else {
        registry.init();
    }

    let ret = run(args).await;
    if let Err(e) = &ret {
        tracing::error!("{e:?}");
    }
    ret.context("Server exited with fatal error")
}

async fn run(args: qotd::Cli) -> anyhow::Result<()> {
    // Get our quotes
    let categories = args.allowed_categories();
    let quotes = qotd::Quotes::from_dir(args.dir, &categories).await?;

    // Start the server
    qotd::Server::new()
        .bind((args.host, args.port))
        .await?
        .drop_privileges(args.user)?
        .serve(quotes)
        .await
}
