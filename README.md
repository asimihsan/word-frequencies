# word-frequencies

Calculate the frequencies of words, pairs of words, etc. in a Wikipedia dataset. One use-case is to create lists of
popular, different words to use in e.g. games, passphrase generation, etc.

## Installation

This crate is not published to crates.io yet so you will need to first [install Rust](https://www.rust-lang.org/tools/install),
clone this repository locally, then run:

```
cargo install --path . --force
```

This will put a `word-frequencies` binary into your `$HOME/.cargo/bin` folder, which you can then put into your `PATH`
environment variable.

## Usage

Run `word-frequencies --help` and e.g. `word-frequencies split --help` for usage instructions. Below is an end-to-end example of
using `word-frequencies` to count unigrams (words) and bigrams (pairs of words), and then calculate the most frequent words.

### 1. Wikipedia dataset download

First download the Wikipedia dataset for the language that you care about.

-   Browse to the Wikipedia CirrusSearch dataset dumps: https://dumps.wikimedia.org/other/cirrussearch/
-   Browse to last complete download. To be safe choose the date before the current one. 
    -   In this example it was https://dumps.wikimedia.org/other/cirrussearch/20200113/
-   Download the "content" file for the language you care about.
    -   In this example English is https://dumps.wikimedia.org/other/cirrussearch/20200113/enwiki-20200113-cirrussearch-content.json.gz
    -   In this example Polish is https://dumps.wikimedia.org/other/cirrussearch/20200113/plwiki-20200113-cirrussearch-content.json.gz

### 2a. Mac and Linux command-line example

Let's assume the Wikipedia dataset is downloaded to `$HOME/datasets/wikipedia/plwiki-20200113-cirrussearch-content.json.gz`.

With this file downloaded, first split the file into multiple pieces, and also Unicode-normalize the input. The output files will be
one line per Wikipedia article.

```
word-frequencies split \
    --input-path $HOME/datasets/wikipedia/plwiki-20200113-cirrussearch-content.json.gz \
    --output-dir $HOME/datasets/wikipedia/plwiki-20200113-split
```

After splitting you can create a frequencies file, which contains counts for unigrams (single words) and bigrams (pairs of words):

```
word-frequencies create-frequencies \
    --input-dir $HOME/datasets/wikipedia/plwiki-20200113-split \
    --output-file $HOME/datasets/wikipedia/plwiki-20200113-split/plwiki-20200113-frequencies.txt \
    --language pl
```

This will create a compressed file `plwiki-20200113-frequencies.txt.gz`. If you `zless` it you can see it contains counts that can
let you build a language model if you'd like.

For now if you only care about the most popular K unigrams, e.g. top 10k words, you can run:

```
word-frequencies top-k-words \
    --number-of-words 10000 \
    --input-file $HOME/datasets/wikipedia/plwiki-20200113-split/plwiki-20200113-frequencies.txt.gz \
    --output-file $HOME/datasets/wikipedia/plwiki-20200113-split/plwiki-20200113-top-10k.txt
```

### 2b. Windows command-line example

TODO, works but need to write out commands and test it


## Dictionary sources

### English

From https://packages.debian.org/sid/wordlist download `wamerican`, `wbritish`, `wcanadian` standard lists
(around 103k words each), then concatenate, sort, de-dupe:

```
cat wamerican/usr/share/dict/american-english \
    wbritish/usr/share/dict/british-english \
    wcanadian/usr/share/dict/canadian-english | sort | uniq > en.txt

```

Note that http://wordlist.aspell.net/12dicts-readme/ is another great resource for curated English words.

### Polish

The Debian `wpolish` dictionary is surprisingly low quality so I scraped Wiktionary to build a Polish dictionary, see below.


### Using Wiktionary

I haven't ironed this out but here is some quick Python code to convert Wiktionary dataset dumps (from the same links as above)
to dictionary files. You can then put these into the "dictionaries" sub-folder and re-run.

```
#!/usr/bin/env python3

import json
import gzip
import unicodedata


def main():
    main_en()
    main_pl()


def main_pl():
    words = []
    with gzip.open("plwiktionary-20200113-cirrussearch-content.json.gz", "rb") as f:
        for line in f:
            data = json.loads(line)
            if "language" not in data:
                continue
            if " " in data["title"]:
                continue
            if "Szablon:język polski" not in data["template"]:
                continue
            word = unicodedata.normalize("NFKC", data["title"])
            words.append(word)
    words.sort()
    with open("pl.txt", "w") as f_out:
        for word in words:
            f_out.write("{0}\n".format(word))


def main_en():
    words = []
    with gzip.open("enwiktionary-20200113-cirrussearch-content.json.gz", "rb") as f:
        for line in f:
            data = json.loads(line)
            if "language" not in data:
                continue
            if " " in data["title"]:
                continue
            if "English" not in data["heading"]:
                continue
            word = unicodedata.normalize("NFKC", data["title"])
            words.append(word)
    words.sort()
    with open("en.txt", "w") as f_out:
        for word in words:
            f_out.write("{0}\n".format(word))


if __name__ == "__main__":
    main()
```

## TODOs

-   Need tests.
-   Option to specify your own dictionary file, that way we don't need to keep adding dictionaries to the binary.
-   Very memory inefficient, need ~15GB RAM for English.
    -   Try interning Strings, I think the string copying is a big culprit.
    -   If still not good enough then use SQLite to count words.
-   Make minimum article count in `create-frequencies` an input parameter.
-   Once crate is published update installation instructions.

## Testing commands for older English dataset

```
word-frequencies split \
    --input-path $HOME/datasets/wikipedia/enwiki-20191202-cirrussearch-content.json.gz \
    --output-dir $HOME/datasets/wikipedia/enwiki-20191202-split
word-frequencies create-frequencies \
    --input-dir $HOME/datasets/wikipedia/enwiki-20191202-split \
    --output-file $HOME/datasets/wikipedia/enwiki-20191202-split/enwiki-20191202-frequencies.txt \
    --language en
word-frequencies top-k-words \
    --number-of-words 10000 \
    --input-file $HOME/datasets/wikipedia/enwiki-20191202-split/enwiki-20191202-frequencies.txt.gz \
    --output-file $HOME/datasets/wikipedia/enwiki-20191202-split/enwiki-20191202-top-10k.txt
echo done
```

## License

`word-frequencies` is distributed under the terms of the Apache License (Version 2.0). See [LICENSE](LICENSE) for
details.
