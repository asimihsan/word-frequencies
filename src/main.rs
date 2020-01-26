extern crate clap;

use clap::{App, AppSettings, Arg, SubCommand};
use std::error::Error;
use std::path::Path;

pub mod create_frequencies;
pub mod split;
pub mod topkwords;
pub mod util;

fn main() -> Result<(), Box<dyn Error>> {
    let app = App::new("Word frequency counter using Wikipedia dataset dumps.")
        .setting(AppSettings::ArgRequiredElseHelp)
        .subcommand(
            SubCommand::with_name("split")
                .about("Split a cirrussearch JSON GZ file into pieces")
                .arg(
                    Arg::with_name("input_path")
                        .long("input-path")
                        .short("p")
                        .required(true)
                        .takes_value(true)
                        .validator(input_path_is_file)
                        .help("Path to cirrussearch JSON GZ file, download from https://dumps.wikimedia.org/other/cirrussearch/")
                        .value_name("FILE"),
                )
                .arg(
                    Arg::with_name("output_dir")
                        .long("output-dir")
                        .short("o")
                        .required(true)
                        .takes_value(true)
                        .help("Output directory for split files. Will be deleted if exists.")
                        .value_name("DIR"),
                )
                .arg(
                    Arg::with_name("pieces")
                        .long("pieces")
                        .short("s")
                        .required(false)
                        .takes_value(true)
                        .validator(validate_pieces)
                        .default_value("12")
                        .help("How many pieces to split the input file into.")
                        .value_name("POSITIVE INTEGER"),
                ))
        .subcommand(
            SubCommand::with_name("create-frequencies")
                .about("Create a frequencies file from line-delimited files of articles")
                .arg(
                    Arg::with_name("input_dir")
                        .long("input-dir")
                        .short("d")
                        .required(true)
                        .takes_value(true)
                        .validator(validate_input_dir)
                        .help("Directory full of line-delimited GZ files. Will put output ARPA language model file here.")
                        .value_name("DIR"),
                )
                .arg(
                    Arg::with_name("output_file")
                        .long("output-file")
                        .short("o")
                        .required(true)
                        .takes_value(true)
                        .help("Name of output ARPA language model file. Will be GZIP compressed and have .gz appended.")
                        .value_name("FILE"),

                )
                .arg(
                    Arg::with_name("language")
                        .long("language")
                        .short("l")
                        .required(true)
                        .takes_value(true)
                        .validator(validate_language_code)
                        .help("Two-character language code for dictionary, e.g. en, pl, etc.")
                        .value_name("ISO 639-1 CODE"),

                ))
        .subcommand(
            SubCommand::with_name("top-k-words")
                .about("Create a file with the top K words (unigrams) in a frequencies file")
                .arg(
                    Arg::with_name("input_file")
                        .long("input-file")
                        .short("f")
                        .required(true)
                        .takes_value(true)
                        .validator(input_path_is_file)
                        .help("GZIP-compressed frequencies file as produced by the 'create-frequencies' sub-command")
                        .value_name("FILE"),
                )
                .arg(
                    Arg::with_name("output_file")
                        .long("output-file")
                        .short("o")
                        .required(true)
                        .takes_value(true)
                        .help("Name of output file to put top K words. Will not be compressed.")
                        .value_name("FILE"),
                )
                .arg(
                    Arg::with_name("number_of_words")
                        .long("number-of-words")
                        .short("k")
                        .required(false)
                        .takes_value(true)
                        .validator(validate_number_of_words)
                        .default_value("10000")
                        .help("Number of words to return, starting with most frequent.")
                        .value_name("POSITIVE INTEGER"),
                )
                .arg(
                    Arg::with_name("minimum_word_length")
                        .long("minimum-word-length")
                        .short("m")
                        .required(false)
                        .takes_value(true)
                        .validator(validate_minimum_word_length)
                        .default_value("3")
                        .help("Minimum (inclusive) length of word to consider.")
                        .value_name("POSITIVE INTEGER"),
                )
        );
    let matches = app.get_matches();

    match matches.subcommand() {
        ("split", Some(split_matches)) => {
            let input_path = Path::new(split_matches.value_of("input_path").unwrap());
            let output_dir = Path::new(split_matches.value_of("output_dir").unwrap());
            let pieces = split_matches
                .value_of("pieces")
                .unwrap()
                .parse::<u32>()
                .unwrap();
            split::handle_split(input_path, output_dir, pieces)
        }
        ("create-frequencies", Some(create_frequencies_matches)) => {
            let input_dir = Path::new(create_frequencies_matches.value_of("input_dir").unwrap());
            let output_file = create_frequencies_matches
                .value_of("output_file")
                .unwrap()
                .to_string();
            let language_code = create_frequencies_matches
                .value_of("language")
                .unwrap()
                .to_string();
            create_frequencies::handle_create_frequencies(input_dir, &output_file, &language_code)
        }
        ("top-k-words", Some(top_k_words_matches)) => {
            let input_file = Path::new(top_k_words_matches.value_of("input_file").unwrap());
            let output_file = Path::new(top_k_words_matches.value_of("output_file").unwrap());
            let minimum_word_length = top_k_words_matches
                .value_of("minimum_word_length")
                .unwrap()
                .parse::<u32>()
                .unwrap();
            let number_of_words = top_k_words_matches
                .value_of("number_of_words")
                .unwrap()
                .parse::<u32>()
                .unwrap();
            topkwords::handle_top_k_words(
                input_file,
                output_file,
                minimum_word_length as usize,
                number_of_words as usize,
            )
        }
        ("", None) => {
            let err: Box<dyn Error> = String::from("Need to specify a sub-command.").into();
            Err(err)
        }
        _ => unreachable!(),
    }
}

fn validate_pieces(input: String) -> Result<(), String> {
    match input.parse::<u32>() {
        Ok(value) => {
            if value == 0 {
                Err(String::from("Pieces cannot be 0."))
            } else if value > 1024 {
                Err(String::from("Pieces too large, must be smaller than 1024."))
            } else {
                Ok(())
            }
        }
        Err(_) => Err(String::from("Pieces is not a valid integer.")),
    }
}

fn validate_number_of_words(input: String) -> Result<(), String> {
    match input.parse::<u32>() {
        Ok(value) => {
            if value == 0 {
                Err(String::from("Number of words cannot be 0."))
            } else if value > 100000 {
                Err(String::from(
                    "Number of words too large, must be smaller than 100,000.",
                ))
            } else {
                Ok(())
            }
        }
        Err(_) => Err(String::from("Pieces is not a valid integer.")),
    }
}

fn validate_minimum_word_length(input: String) -> Result<(), String> {
    match input.parse::<u32>() {
        Ok(value) => {
            if value == 0 {
                Err(String::from("Minimum word length cannot be 0."))
            } else {
                Ok(())
            }
        }
        Err(_) => Err(String::from("Pieces is not a valid integer.")),
    }
}

fn input_path_is_file(input: String) -> Result<(), String> {
    if Path::new(&input).is_file() {
        Ok(())
    } else {
        Err(String::from(
            "Input filepath does not exist or isn't a file.",
        ))
    }
}

fn validate_input_dir(input: String) -> Result<(), String> {
    if Path::new(&input).is_dir() {
        Ok(())
    } else {
        Err(String::from(
            "Input path doesn't exist or isn't a directory.",
        ))
    }
}

fn validate_language_code(input: String) -> Result<(), String> {
    match input.as_str() {
        "en" | "pl" => Ok(()),
        _ => Err(String::from(
            "Unsupported dictionary language code. Currently support ['en', 'pl']",
        )),
    }
}
