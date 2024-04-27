//! Module for processing command-line arguments
#![cfg(feature = "cli")]

use std::path::PathBuf;

use clap::Parser;

use crate::{AllowedCategories, QuoteCategory};

/// A Quote of the Day Protocol (RFC 865) server
#[derive(Debug, Parser)]
pub struct Cli {
    /// Choose from all available quotes, both offensive and not (see --categories)
    #[arg(short)]
    all: bool,

    /// Allowed quote categories
    ///
    /// Short-form options are available as well: -a is equivalent to `--categories all`, while -o is equivalent to `--categories offensive`.
    /// If none are selected, the default is to only choose the decorous (i.e. inoffensive) quotes.
    /// This option, if provided, supersedes the others; otherwise, -a will supersede -o.
    #[arg(long, short, value_enum)]
    categories: Option<AllowedCategories>,

    /// Directory to read quote files from
    ///
    /// Quote files are expected to be simple text files. Individual quotes may contain multiple lines;
    /// lines beginning with the '%' character are treated as the quote delimiters, and otherwise ignored.
    /// If the file name ends with "-o" it is considered to contain offensive quotes, otherwise it is
    /// assumed to only contain generally acceptable, "clean" quotes; see the --categories option.
    /// If the file contains the token "$SerrOFQ$", it is assumed that all alphabetic characters have been
    /// rot-13 encoded; if this token is not present, or if the token "$FreeBSD$" is encountered first, the
    /// file is assumed to not be encoded.
    #[arg(long, short, default_value = default_dir().into_os_string(), value_hint = clap::ValueHint::DirPath)]
    pub dir: PathBuf,

    /// Address to bind to
    #[arg(
        long,
        short = 'i',
        default_value = "127.0.0.1",
        value_name = "IP or HOSTNAME"
    )]
    pub host: String,

    /// Choose only from offensive quotes (see --categories)
    #[arg(short)]
    offensive: bool,

    /// Port to listen on
    #[arg(long, short, default_value_t = 17)]
    pub port: u16,

    /// Increase verbosity
    #[arg(short, long = "verbose", action = clap::ArgAction::Count)]
    pub verbosity: u8,
}

impl Cli {
    pub fn allowed_categories(&self) -> Vec<QuoteCategory> {
        if let Some(categories) = self.categories {
            categories.as_category_vec()
        } else if self.all {
            AllowedCategories::All.as_category_vec()
        } else if self.offensive {
            AllowedCategories::Offensive.as_category_vec()
        } else {
            AllowedCategories::Decorous.as_category_vec()
        }
    }
}

fn default_dir() -> PathBuf {
    let mut path = std::env::var("CARGO_MANIFEST_DIR")
        .map(|p| p.into())
        .unwrap_or_else(|_| {
            let mut path = std::env::current_exe().unwrap();
            path.pop();
            path
        });
    path.push("data");

    path
}
