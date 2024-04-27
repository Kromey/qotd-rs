//! Core library for qotd-rs

use std::path::Path;

mod args;
#[cfg(feature = "cli")]
pub use args::*;
mod quotes;
pub use quotes::*;
mod server;
pub use server::*;
use tokio::net::ToSocketAddrs;

pub async fn serve_dir<
    A: ToSocketAddrs + std::fmt::Debug,
    P: AsRef<Path> + Send + std::fmt::Debug + 'static,
>(
    addr: A,
    dir: P,
) -> anyhow::Result<()> {
    let quotes = Quotes::from_dir(dir, &[QuoteCategory::Decorous]).await?;
    Server::new()
        .bind(addr)
        .await?
        .drop_privileges("nobody")?
        .serve(quotes)
        .await
}
