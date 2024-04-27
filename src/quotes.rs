//! This module is responsible for parsing quote files

use std::path::Path;

use futures::{future::BoxFuture, FutureExt};
use rand::{thread_rng, Rng};
use rand_distr::{Distribution, WeightedAliasIndex};
use tokio::{
    fs::{read_dir, File},
    io::{self, AsyncBufReadExt, AsyncReadExt, AsyncSeekExt, BufReader},
};
use tracing::{info, instrument};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum QuoteCategory {
    #[default]
    Decorous,
    Offensive,
}

const SEPARATOR: &str = "%";
const ROT31_TOKEN: &str = "$SerrOFQ$";
const PLAIN_TOKEN: &str = "$FreeBSD$";
const OFFENSIVE_SUFFIX: &str = "-o";

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum FileEncoding {
    #[default]
    Plain,
    Rot13,
}

#[derive(Debug, Default, Clone, Copy)]
struct QuoteIndex {
    offset: u64,
    length: usize,
}

#[derive(Debug)]
struct QuoteFile {
    file_handle: File,
    quotes: Vec<QuoteIndex>,
    encoding: FileEncoding,
    category: QuoteCategory,
}

#[derive(Debug)]
pub struct Quotes {
    files: Vec<QuoteFile>,
    file_weights: WeightedAliasIndex<usize>,
}

impl Quotes {
    #[instrument]
    pub fn from_dir<P: AsRef<Path> + Send + std::fmt::Debug + 'static>(
        dir: P,
        allowed_categories: &[QuoteCategory],
    ) -> BoxFuture<'_, io::Result<Self>> {
        async move {
            let mut files = Vec::new();

            let mut entries = read_dir(dir).await?;
            while let Some(entry) = entries.next_entry().await? {
                if entry.file_type().await?.is_dir() {
                    files.append(
                        &mut Self::from_dir(entry.path(), allowed_categories)
                            .await?
                            .files,
                    );
                } else if entry.file_type().await?.is_file() {
                    let file = Self::process_file(entry.path()).await?;
                    if allowed_categories.contains(&file.category) && !file.quotes.is_empty() {
                        info!(
                            "Indexed file \"{}\" containing {} entries",
                            entry.path().to_str().unwrap(),
                            file.quotes.len()
                        );
                        files.push(file);
                    } else {
                        info!(
                            "File \"{}\" is not in allowed categories",
                            entry.path().to_str().unwrap()
                        );
                    }
                }
            }

            // Prepare a weighted distribution to ensure fair selection of every quote, regardless of file sizes
            let file_weights =
                WeightedAliasIndex::new(files.iter().map(|file| file.quotes.len()).collect())
                    .unwrap();

            Ok(Self {
                files,
                file_weights,
            })
        }
        .boxed()
    }

    async fn process_file<P: AsRef<Path>>(path: P) -> io::Result<QuoteFile> {
        let path = path.as_ref();

        let category = if path
            .to_str()
            .unwrap_or(OFFENSIVE_SUFFIX)
            .ends_with(OFFENSIVE_SUFFIX)
        {
            QuoteCategory::Offensive
        } else {
            QuoteCategory::Decorous
        };

        let fh = File::open(path).await?;
        let mut buf_read = BufReader::new(fh);

        let mut offset = 0;
        let mut last_offset = 0;

        // Start with a large capacity to reduce reallocations
        let mut quotes = Vec::with_capacity(0xFFF);
        let mut encoding = FileEncoding::Plain;
        let mut encoding_found = false;

        // Initialize a large capacity for the buffer to avoid reallocations
        let mut line_buf = String::with_capacity(0xFF);

        while buf_read.read_line(&mut line_buf).await? > 0 {
            if !encoding_found {
                if line_buf.contains(ROT31_TOKEN) {
                    encoding = FileEncoding::Rot13;
                    encoding_found = true;
                } else if line_buf.contains(PLAIN_TOKEN) {
                    encoding = FileEncoding::Plain;
                    encoding_found = true;
                }
            }

            let line_len = line_buf.len();
            if line_buf.starts_with(SEPARATOR) {
                let len = offset - last_offset;
                if len > 0 {
                    quotes.push(QuoteIndex {
                        offset: last_offset as u64,
                        length: len,
                    });
                }
                last_offset = offset + line_len;
            }
            offset += line_len;
            line_buf.clear();
        }

        // No need to maintain extra capacity after this point, as the data should remain static
        quotes.shrink_to_fit();

        Ok(QuoteFile {
            file_handle: buf_read.into_inner(),
            quotes,
            encoding,
            category,
        })
    }

    pub async fn random_quote(&mut self) -> io::Result<Vec<u8>> {
        // We have to select an index, rather than using `rand`'s SliceSequence trait, to avoid
        // holding the non-`Send` RNG across awaits - although I'm sure there's a way around that
        let i = self.file_weights.sample(&mut thread_rng());
        self.read_quote(i).await
    }

    pub async fn read_quote(&mut self, file_index: usize) -> io::Result<Vec<u8>> {
        let file = &mut self.files[file_index];
        // @see RNG note in `Self::random_quote`
        let i = thread_rng().gen_range(0..file.quotes.len());

        let quote_index = file.quotes[i];
        file.file_handle
            .seek(io::SeekFrom::Start(quote_index.offset))
            .await?;
        let mut quote = vec![0_u8; quote_index.length];
        file.file_handle.read_exact(&mut quote).await?;

        if self.files[file_index].encoding == FileEncoding::Rot13 {
            Self::rot13(&mut quote);
        }

        Ok(quote)
    }

    fn rot13(text: &mut [u8]) {
        text.iter_mut().for_each(|c| match c {
            b'A'..=b'M' | b'a'..=b'm' => *c += 13,
            b'N'..=b'Z' | b'n'..=b'z' => *c -= 13,
            _ => {}
        });
    }
}
