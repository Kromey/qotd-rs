//! Module for processing command-line arguments
#![cfg(feature = "cli")]

use std::path::PathBuf;

use clap::Parser;

use crate::{AllowedCategories, QuoteCategory};

/// A Quote of the Day Protocol (RFC 865) server
#[derive(Debug, Parser)]
#[command(version, about, next_line_help = true)]
pub struct Cli {
    /// Choose from all available quotes, both offensive and not (see --categories)
    #[arg(long, short)]
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

    /// If present, log all output to the provided file
    #[arg(long, short, value_hint = clap::ValueHint::FilePath)]
    pub log_file: Option<PathBuf>,

    /// Choose only from offensive quotes (see --categories)
    #[arg(long, short)]
    offensive: bool,

    /// Port to listen on
    #[arg(long, short, default_value_t = 17)]
    pub port: u16,

    /// Reduce output
    ///
    /// This option is ignored if any number of --verbose flags are present
    #[arg(long, short)]
    quiet: bool,

    /// Increase verbosity
    ///
    /// This flag may appear multiple times, each appearance (up to 3) increasing the level of verbosity
    #[arg(short, long = "verbose", action = clap::ArgAction::Count)]
    verbosity: u8,
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

    pub fn verbosity(&self) -> tracing::level_filters::LevelFilter {
        match self.verbosity {
            0 => {
                if self.quiet {
                    tracing::Level::ERROR
                } else {
                    tracing::Level::WARN
                }
            }
            1 => tracing::Level::INFO,
            2 => tracing::Level::DEBUG,
            _ => tracing::Level::TRACE,
        }
        .into()
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
