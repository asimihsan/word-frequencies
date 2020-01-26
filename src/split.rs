use flate2::read::GzDecoder;
use flate2::Compression;
use flate2::GzBuilder;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::error::Error;
use std::fs;
use std::fs::File;
use std::io::{BufRead, BufWriter};
use std::io::{BufReader, Write};
use std::path::Path;
use unicode_normalization::UnicodeNormalization;

pub fn handle_split(
    input_path: &Path,
    output_dir: &Path,
    pieces: u32,
) -> Result<(), Box<dyn Error>> {
    println!("handle_split entry");

    if output_dir.is_dir() {
        println!("deleting output directory {}", output_dir.to_string_lossy());
        fs::remove_dir_all(output_dir)?;
    }
    fs::create_dir(output_dir)?;

    let mut output_files = Vec::with_capacity(pieces as usize);
    let basename = input_path.file_stem().unwrap().to_string_lossy();
    for i in 0..pieces {
        let output_filename = format!("{}.split.{:03}", basename, i);
        let output_filename_gz = format!("{}.gz", output_filename);
        let output_path = Path::join(output_dir, output_filename_gz);
        let output_file = File::create(&output_path).unwrap_or_else(|err| {
            panic!(
                "Could not create output file {:?} due to {:?}",
                output_path, err
            )
        });
        let output_file = BufWriter::with_capacity(1024 * 1024, output_file);
        let output_file = GzBuilder::new()
            .filename(output_filename)
            .write(output_file, Compression::best());
        output_files.push(output_file);
    }

    let mut rng: StdRng = SeedableRng::seed_from_u64(42);
    let reader = File::open(input_path)?;
    let reader = GzDecoder::new(reader);
    let reader = BufReader::new(reader);
    let mut i = 0;
    for line in reader.lines() {
        let line = line.unwrap();
        let line_json: serde_json::Value = serde_json::from_str(line.as_str()).unwrap();
        let text = line_json.get("text");
        if text.is_none() {
            continue;
        }
        let text = text.unwrap().as_str().unwrap();
        let text = text.nfkc().collect::<String>();
        let random_piece = rng.gen_range(0, pieces) as usize;
        let output_file = &mut output_files[random_piece];
        output_file.write_all(text.as_bytes())?;
        output_file.write_all(b"\n")?;

        i += 1;
        if i % 10000 == 0 {
            println!("{}", i);
        }
    }

    for output_file in output_files {
        let mut inner = output_file.finish()?;
        inner.flush()?;
    }

    Ok(())
}
