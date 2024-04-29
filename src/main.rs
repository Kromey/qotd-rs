use clap::Parser;

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    let args = qotd_rs::Cli::parse();

    // Set up our logging
    tracing::subscriber::set_global_default(
        tracing_subscriber::fmt()
            .with_max_level(args.verbosity())
            .finish(),
    )?;

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
