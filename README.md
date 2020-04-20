# hyperpolyglot
A fast programming language detector. Hyperpolyglot is a port of Github's [Linguist](https://github.com/github/linguist) Ruby library to Rust. Hyperpolyglot supports detecting the programming language of a file or detecting the programming language makeup of a directory. For more details on how the language detection is done, see the linguist [README](https://github.com/github/linguist/blob/master/README.md).

### CLI
** Installing **

`cargo install hyperpolyglot`

** Usage **

`hyply --help`

### Library
** Adding as a dependency **

```TOML
[dependencies]
hyperpolyglot = "0.1.0"
```

** Detect **

```Rust
use hyperpolyglot;

let detection = hyperpolyglot::detect(Path::new("src/bin/main.rs"));
assert_eq!(Ok(Some(Detection::Heuristics("Rust"))), detection);
```

** Breakdown **
```Rust
use hyperpolyglot::{get_language_breakdown};

let breakdown: HashMap<&'static str, Vec<(Detection, PathBuf)>> = get_language_breakdown("src/");
println!("{:?}", breakdown.get("Rust"));
```

### Divergences from Linguist
* Less meticulous tokenization. Hyperpolyglot currently doesn't filter out comments and string literals.

* The probability of the language occuring is not taken into account when classifying. All languages are assumed to have equal probability.

* An additional heuristic was added for .h files.

* Vim and Emacs modelines are not considered in the detection process.

* Generated and Binary files are not excluded from the breakdown function.

* When calculating the language makeup of a directory, file count is used instead of byte count.

### Benchmarks
* TODO: add later
