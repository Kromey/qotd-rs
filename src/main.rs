use std::{fs::File, sync::Mutex};

use anyhow::Context;
use clap::Parser;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, Layer};

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    let args = qotd_rs::Cli::parse();

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
                    .with_filter(args.verbosity()),
            )
            .init();
    } else {
        registry.init();
    }

    // Get our quotes
    let categories = args.allowed_categories();
    let quotes = qotd_rs::Quotes::from_dir(args.dir, &categories).await?;

    // Start the server
    qotd_rs::Server::new()
        .bind((args.host, args.port))
        .await?
        .drop_privileges("nobody")?
        .serve(quotes)
        .await
}
