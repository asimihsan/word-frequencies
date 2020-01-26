use std::cmp::max;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::error::Error;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc;

use flate2::write::GzEncoder;
use flate2::{Compression, GzBuilder};
use scoped_threadpool::Pool;

use crate::util::{get_dictionary, LineIterator, OUT_OF_VOCABULARY_WORD};

/// Minimum number of articles that a word must be in so that it is included in the counts.
const MINIMUM_ARTICLE_THRESHOLD: u64 = 40;

/// References
/// -   https://rust-lang-nursery.github.io/rust-cookbook/concurrency/threads.html
pub fn handle_create_frequencies(
    input_dir: &Path,
    output_file: &String,
    language_code: &String,
) -> Result<(), Box<dyn Error>> {
    println!("handle_create_frequencies entry");

    let dictionary = get_dictionary(language_code)?;
    println!("calculating ngrams...");
    let ngrams = calculate_ngrams_threaded(input_dir, &dictionary);
    ngrams.persist_to_file(input_dir, output_file)?;

    Ok(())
}

impl NgramsResult {
    fn persist_to_file(
        &self,
        output_dir: &Path,
        output_file: &String,
    ) -> Result<(), Box<dyn Error>> {
        let gzip_output_filepath = NgramsResult::get_gzip_output_filename(output_dir, output_file);
        println!(
            "NgramsResult writing frequencies to {:?}...",
            gzip_output_filepath
        );
        let mut output_file =
            NgramsResult::get_gzip_output_file(output_file, &gzip_output_filepath);
        writeln!(&mut output_file, "\\data\\")?;
        writeln!(&mut output_file, "total unigrams = {}", self.total_unigrams)?;
        writeln!(&mut output_file, "ngram 1 = {}", self.unigram_counts.len())?;
        writeln!(&mut output_file, "ngram 2 = {}", self.bigram_counts.len())?;
        writeln!(&mut output_file)?;
        writeln!(&mut output_file, "\\1-grams:")?;
        for (token, count) in self.unigram_counts.iter() {
            if *self
                .unigram_article_counts
                .get(token)
                .unwrap_or(&u64::max_value())
                > MINIMUM_ARTICLE_THRESHOLD
            {
                writeln!(&mut output_file, "{}\t{}", count, token)?;
            }
        }
        writeln!(&mut output_file)?;
        writeln!(&mut output_file, "\\2-grams:")?;
        for ((token1, token2), count) in self.bigram_counts.iter() {
            if *self
                .unigram_article_counts
                .get(token1)
                .unwrap_or(&u64::max_value())
                > MINIMUM_ARTICLE_THRESHOLD
                && *self
                    .unigram_article_counts
                    .get(token2)
                    .unwrap_or(&u64::max_value())
                    > MINIMUM_ARTICLE_THRESHOLD
            {
                writeln!(&mut output_file, "{}\t{}\t{}", count, token1, token2)?;
            }
        }
        writeln!(&mut output_file)?;
        writeln!(&mut output_file, "\\end\\")?;

        Ok(())
    }

    fn get_gzip_output_filename(output_dir: &Path, output_file: &String) -> PathBuf {
        let output_file_path = Path::new(output_file);
        let output_file_extension = output_file_path
            .extension()
            .unwrap_or_default()
            .to_str()
            .unwrap()
            .to_string();
        let output_file_path =
            output_file_path.with_extension(format!("{}.gz", output_file_extension));
        output_dir.join(output_file_path)
    }

    fn get_gzip_output_file(
        original_output_file: &String,
        gzip_output_filepath: &PathBuf,
    ) -> BufWriter<GzEncoder<File>> {
        let gzip_output_file = File::create(gzip_output_filepath).unwrap_or_else(|err| {
            panic!(
                "Could not create output file {:?} due to {:?}",
                gzip_output_filepath, err
            )
        });
        let gzip_output_file = GzBuilder::new()
            .filename(original_output_file.as_str())
            .write(gzip_output_file, Compression::best());
        BufWriter::new(gzip_output_file)
    }
}

fn merge_ngrams_results(iter: impl Iterator<Item = NgramsResult>) -> NgramsResult {
    let mut total_unigrams = 0;
    let mut unigram_counts = BTreeMap::new();
    let mut unigram_article_counts = HashMap::new();
    let mut bigram_counts = BTreeMap::new();
    for result in iter {
        total_unigrams += result.total_unigrams;

        for (word, count) in result.unigram_counts.into_iter() {
            let existing_count = unigram_counts.entry(word).or_insert(0);
            *existing_count += count;
        }

        for (word, count) in result.unigram_article_counts.into_iter() {
            let existing_count = unigram_article_counts.entry(word).or_insert(0);
            *existing_count += count;
        }

        for ((word1, word2), count) in result.bigram_counts.into_iter() {
            let existing_count = bigram_counts.entry((word1, word2)).or_insert(0);
            *existing_count += count;
        }
    }
    NgramsResult {
        total_unigrams,
        unigram_counts,
        unigram_article_counts,
        bigram_counts,
    }
}

fn calculate_ngrams_threaded(input_dir: &Path, dict: &HashSet<String>) -> NgramsResult {
    let mut pool = Pool::new(max(num_cpus::get() as u32 - 1, 1));
    let (tx, rx) = mpsc::channel();
    pool.scoped(|scope| {
        input_dir
            .read_dir()
            .unwrap()
            .map(|entry| entry.unwrap())
            .map(|entry| entry.path())
            .filter(|path| path.is_file())
            .filter(|path| {
                path.file_stem()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .contains("split")
            })
            .for_each(|input_file| {
                let tx = tx.clone();
                scope.execute(move || {
                    let result = calculate_ngrams(input_file.as_ref(), dict);
                    if result.is_ok() {
                        tx.send(result.unwrap()).unwrap();
                    } else {
                        panic!(
                            "failed to determine twogram counts for file {:?}: {:?}",
                            input_file, result
                        );
                    }
                });
            });
    });
    drop(tx);
    merge_ngrams_results(rx.iter())
}

#[derive(Debug)]
struct NgramsResult {
    /// Total number of unigrams in the corpus. The probability of a given unigram is the frequency
    /// of the unigram divided by this.
    total_unigrams: u64,

    /// Counts of specific unigrams. When you divide this by total_unigrams you get the
    /// unigram probability. If a unigram occurs more than once in a given article it is incremented
    /// more than once in this count.
    unigram_counts: BTreeMap<String, u64>,

    /// Number of articles that a given unigram is in. If a unigfram occurs more than once in a
    /// given article then this count is increment by 1 only.
    unigram_article_counts: HashMap<String, u64>,

    /// Counts of specific bigrams. The probability of a bigram (w_1, w_2) is the count of
    /// (w_1, w_2) divided by the count of w_1, which you can get from unigram_counts.
    bigram_counts: BTreeMap<(String, String), u64>,
}

fn calculate_ngrams(
    input_file: &Path,
    dict: &HashSet<String>,
) -> Result<NgramsResult, std::io::Error> {
    let mut total_unigrams = 0;
    let mut unigram_counts = BTreeMap::new();
    let mut unigram_article_counts = HashMap::new();
    let mut bigram_counts = BTreeMap::new();
    for line in LineIterator::new(input_file).unwrap() {
        let line_borrowed = line.borrow();
        let tokens: Vec<&str> = line_borrowed
            .split_whitespace()
            .map(|token| {
                token.trim_matches(|c: char| c.is_ascii_punctuation() || c.is_whitespace())
            })
            .map(|token| {
                if dict.contains(token) {
                    token
                } else {
                    OUT_OF_VOCABULARY_WORD
                }
            })
            .collect();
        let mut seen_unigrams = HashSet::new();
        for (token1, token2) in tokens.iter().zip(tokens.iter().skip(1)) {
            total_unigrams += 1;

            let unigram_entry = unigram_counts.entry((*token1).to_string()).or_insert(0);
            *unigram_entry += 1;

            seen_unigrams.insert(*token1);

            let bigram_entry = bigram_counts
                .entry(((*token1).to_string(), (*token2).to_string()))
                .or_insert(0);
            *bigram_entry += 1;
        }

        // The iteration above missed the last token as a unigram so we tack it on here.
        if tokens.len() >= 2 {
            let last_token = tokens[tokens.len() - 1];
            total_unigrams += 1;
            let unigram_entry = unigram_counts.entry(last_token.to_string()).or_insert(0);
            *unigram_entry += 1;
        }

        for unigram in seen_unigrams {
            let unigram_article_entry = unigram_article_counts
                .entry((*unigram).to_string())
                .or_insert(0);
            *unigram_article_entry += 1;
        }
    }
    Ok(NgramsResult {
        total_unigrams,
        unigram_counts,
        unigram_article_counts,
        bigram_counts,
    })
}
