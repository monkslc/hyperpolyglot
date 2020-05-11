# hyperpolyglot
### A fast programming language detector.
Hyperpolyglot is a fast programming language detector written in Rust based on Github's [Linguist](https://github.com/github/linguist) Ruby library. Hyperpolyglot supports detecting the programming language of a file or detecting the programming language makeup of a directory. For more details on how the language detection is done, see the [Linguist](https://github.com/github/linguist) [README](https://github.com/github/linguist/blob/master/README.md).

### CLI
**Installing**
`cargo install hyperpolyglot`

**Usage**
`hyply [PATH]`

**Output**
```
85.00% Rust
15.00% RenderScript
```

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
|hyperpolyglot (multi-threaded) |1,188    |1,186      |1,166   |1,226   |
|hyperpolyglot (single-threaded)|2,424    |2,424      |2,414   |2,442   |
|enry                           |21,619   |21,566     |21,514  |21,855  |
|Linguist                       |42,407   |42,386     |42,070  |42,856  |

**[Rust](https://github.com/rust-lang/rust) Repo**

|Tool                           |mean (ms)|median (ms)|min (ms)|max (ms)|
|-------------------------------|---------|-----------|--------|--------|
|hyperpolyglot (multi-threaded) |3,808    |3,751      |3,708   |4,253   |
|hyperpolyglot (single-threaded)|8,341    |8,334      |8,276   |8,437   |
|enry                           |82,300   |82,215     |82,021  |82,817  |
|Linguist                       |196,780  |197,300    |194,033 |202,930 |

**[Linux](https://github.com/torvalds/linux) Kernel**
* The reason hyperpolyglot is so much faster here is the heuristic added to .h files which significantly speeds up detection for .h files that can't be classified with the Objective-C or C++ heuristics

|Tool                           |mean (s)|median (s)|min (s) |max (s) |
|-------------------------------|---------|---------|------- |------- |
|hyperpolyglot (multi-threaded) |3.7574   |3.7357   |3.7227  |3.9021  |
|hyperpolyglot (single-threaded)|7.5833   |7.5683   |7.5445  |7.6489  |
|enry                           |137.6046 |137.4229 |137.1955|138.8694|


### Accuracy
All of the programming language detectors are far from perfect and hyperpolyglot is no exception. It's language detections mirror [Linguist](https://github.com/github/linguist) and [enry](https://github.com/go-enry/go-enry) for most files with the biggest divergences coming from files that need to fall back on the classifier. Files that can be detected through a common known filename, an extension, or by following the set of [heuristics](https://github.com/monkslc/hyperpolyglot/blob/master/heuristics.yml) should approach 100% accuracy.

## License

Licensed under either of

 * Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license
   ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
