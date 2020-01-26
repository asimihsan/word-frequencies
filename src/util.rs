use flate2::read::GzDecoder;
use std::cell::RefCell;
use std::collections::HashSet;
use std::error::Error;
use std::ffi::OsStr;
use std::fs::File;
use std::io;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::rc::Rc;
use unicode_normalization::UnicodeNormalization;

/// If a word is not in the dictionry change it to this. This will never appear in the corpus
/// because we trim puncutation from the beginning and ends of words.
pub const OUT_OF_VOCABULARY_WORD: &str = "<unk>";

pub struct LineIterator {
    reader: Box<dyn BufRead>,
    buf: Rc<RefCell<String>>,
}

impl LineIterator {
    pub fn new(input_file: &Path) -> Result<LineIterator, Box<dyn Error>> {
        let file = File::open(input_file).unwrap();
        match input_file.extension().and_then(OsStr::to_str) {
            Some("gz") => {
                let file = GzDecoder::new(file);
                let file = BufReader::new(file);
                Ok(LineIterator {
                    reader: Box::new(file),
                    buf: Rc::new(RefCell::new(String::new())),
                })
            }
            _ => {
                let file = BufReader::new(file);
                Ok(LineIterator {
                    reader: Box::new(file),
                    buf: Rc::new(RefCell::new(String::new())),
                })
            }
        }
    }
}

impl Iterator for LineIterator {
    type Item = Rc<RefCell<String>>;

    fn next(&mut self) -> Option<Self::Item> {
        self.buf.borrow_mut().clear();
        match self.reader.read_line(&mut self.buf.borrow_mut()) {
            Ok(0) => None,
            Ok(_) => Some(Rc::clone(&self.buf)),
            Err(_) => None,
        }
    }
}

const EN_DICT: &[u8] = include_bytes!("dictionaries/en.txt");
const PL_DICT: &[u8] = include_bytes!("dictionaries/pl.txt");

pub fn get_dictionary(language_code: &str) -> Result<HashSet<String>, Box<dyn Error>> {
    let dict_bytes = match language_code {
        "en" => Ok(EN_DICT),
        "pl" => Ok(PL_DICT),
        _ => {
            let err: Box<dyn Error> =
                format!("No dictionary available for language {}", language_code).into();
            Err(err)
        }
    };
    let dict = io::Cursor::new(dict_bytes?);
    let dict = BufReader::new(dict);
    let dict = dict
        .lines()
        .map(|result| result.unwrap())
        .map(|line| line.nfkc().collect::<String>())
        .filter(|line| !line.starts_with('#'))
        .map(|line| {
            String::from(line.trim_matches(|c: char| c.is_ascii_punctuation() || c.is_whitespace()))
        })
        .filter(|line| !line.is_empty())
        .collect();
    Ok(dict)
}
