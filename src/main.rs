use clap::Parser;
use tracing_subscriber::FmtSubscriber;

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    let args = qotd_rs::Cli::parse();

    // Set up our logging
    tracing::subscriber::set_global_default(FmtSubscriber::new())?;

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
