use std::error::Error;
use std::path::Path;

use crate::util::{LineIterator, OUT_OF_VOCABULARY_WORD};
use std::cmp::Reverse;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::ops::Deref;

pub fn handle_top_k_words(
    input_file: &Path,
    output_file: &Path,
    minimum_word_length: usize,
    number_of_words: usize,
) -> Result<(), Box<dyn Error>> {
    let onegrams = load_sorted_onegrams(input_file).unwrap();
    let top_onegrams: Vec<String> = onegrams
        .into_iter()
        .map(|(word, _count)| word)
        .filter(|word| word.len() >= minimum_word_length)
        .take(number_of_words)
        .collect();
    write_sorted_onegrams_to_file(top_onegrams, output_file).unwrap();
    Ok(())
}

fn load_sorted_onegrams(input_file: &Path) -> Result<Vec<(String, u64)>, Box<dyn Error>> {
    let mut result = Vec::new();
    let mut loading_onegrams = false;
    for line in LineIterator::new(input_file).unwrap() {
        let line_borrowed = line.borrow();
        let line_borrowed = line_borrowed.deref();
        if line_borrowed.starts_with("\\1-grams:") {
            loading_onegrams = true;
            continue;
        }
        if !loading_onegrams {
            continue;
        }
        if line_borrowed.trim_end().is_empty() {
            break;
        }
        let elems: Vec<&str> = line_borrowed.split("\t").collect();
        let count: u64 = elems[0].parse().expect("Needed a number");
        let token = elems[1].trim_end();
        if token == OUT_OF_VOCABULARY_WORD {
            continue;
        }
        result.push((token.to_string(), count));
    }
    result.sort_by_key(|(_word, count)| Reverse(*count));

    Ok(result)
}

fn write_sorted_onegrams_to_file(
    top_onegrams: Vec<String>,
    output_file_path: &Path,
) -> Result<(), Box<dyn Error>> {
    let output_file = File::create(output_file_path).unwrap_or_else(|err| {
        panic!(
            "Could not create output file {:?} due to {:?}",
            output_file_path, err
        )
    });
    let mut output_file = BufWriter::new(output_file);
    for onegram in top_onegrams {
        output_file.write_all(onegram.as_bytes())?;
        output_file.write_all(b"\n")?;
    }
    Ok(())
}
