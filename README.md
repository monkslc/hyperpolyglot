# hyperpolyglot
### A fast programming language detector.
Hyperpolyglot is a port of Github's [Linguist](https://github.com/github/linguist) Ruby library to Rust. Hyperpolyglot supports detecting the programming language of a file or detecting the programming language makeup of a directory. For more details on how the language detection is done, see the linguist [README](https://github.com/github/linguist/blob/master/README.md).

### CLI
**Installing**
`cargo install hyperpolyglot`

**Usage**
`hyply --help`

### Library
**Adding as a dependency**
```TOML
[dependencies]
hyperpolyglot = "0.1.0"
```

**Detect**
```Rust
use hyperpolyglot;

let detection = hyperpolyglot::detect(Path::new("src/bin/main.rs"));
assert_eq!(Ok(Some(Detection::Heuristics("Rust"))), detection);
```

**Breakdown**
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
* Benchmarks were run using the command line tool [hyperfine](https://github.com/sharkdp/hyperfine)
* Benchmarks were run on a 8gb 3.1 GHz Dual-Core Intel Core i5 MacBook Pro
* [enry](https://github.com/go-enry/go-enry) is a port of the [Linguist](https://github.com/github/linguist) library to go
* Both [enry](https://github.com/go-enry/go-enry) and [Linguist](https://github.com/github/linguist) are single-threaded

**[samples](https://github.com/monkslc/hyperpolyglot/tree/master/samples) dir**
|Tool                           |mean (ms)|median (ms)|min (ms)|max (ms)|
|-------------------------------|---------|-----------|--------|--------|
|hyperpolyglot (multi-threaded) |1,208    |1,210      |1,181   |1,247   |
|hyperpolyglot (single-threaded)|2,472    |2,467      |2,421   |2,561   |
|enry                           |21,653   |21,641     |21,552  |21,800  |
|Linguist                       |42,510   |42,527     |42,372  |42,680  |

**[Rust](https://github.com/rust-lang/rust) Repo**
|Tool                           |mean (ms)|median (ms)|min (ms)|max (ms)|
|-------------------------------|---------|-----------|--------|--------|
|hyperpolyglot (multi-threaded) |3,958    |3,918      |3,788   |4,201   |
|hyperpolyglot (single-threaded)|8,795    |8,770      |8,689   |9,011   |
|enry                           |85,810   |83,779     |82,980  |101,145 |
|Linguist                       |200,050  |200,118    |198,803 |203,218 |

### Accuracy
All of the programming language detectors are far from perfect and hyperpolyglot is no exception. It's language detections mirror [Linguist](https://github.com/github/linguist) and [enry](https://github.com/go-enry/go-enry) for most files. The biggest differences occur in files that rely on the classifier. Files that can be detected through a common known filename, an extension, or by following the set of [heuristics](https://github.com/monkslc/hyperpolyglot/blob/master/heuristics.yml) should approach 100% accuracy.
