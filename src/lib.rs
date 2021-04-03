//! # Hyperpolyglot
//! `hyperpolyglot` is a fast programming language detector.

use ignore::{overrides::OverrideBuilder, WalkBuilder};
use std::{
    collections::HashMap,
    convert::TryFrom,
    env, fmt,
    fs::File,
    io::{BufReader, Read, Seek, SeekFrom},
    path::{Path, PathBuf},
    sync::mpsc,
};

pub mod detectors;
pub mod filters;

// Include the map that stores language info
// static LANGUAGE_INFO: phf::Map<&'static str, Language> = ...;
include!("codegen/language-info-map.rs");

const MAX_CONTENT_SIZE_BYTES: usize = 51200;

/// The language struct that contains the name and other interesting information about a
/// language.
///
/// # Examples
/// ```
/// use hyperpolyglot::{Language, LanguageType};
/// use std::convert::TryFrom;
///
/// let language = Language::try_from("Rust").unwrap();
/// let expected = Language {
///     name: "Rust",
///     language_type: LanguageType::Programming,
///     color: Some("#dea584"),
///     group: None,
/// };
/// assert_eq!(language, expected)
/// ```
///
/// # Errors
/// `try_from` will error if the langauge name is not one of the known languages
///
/// If try_from is called with a language returned from [`detect`] or [`get_language_breakdown`]
/// the value is guaranteed to be there and can be unwrapped
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Language {
    /// The name of the language
    pub name: &'static str,
    /// Type of language. ex/ Data, Programming, Markup, Prose
    pub language_type: LanguageType,
    /// The css hex color used to represent the language on github. ex/ #dea584
    pub color: Option<&'static str>,
    /// Name of the parent language. ex/ The group for TSX would be TypeScript
    pub group: Option<&'static str>,
}

impl TryFrom<&str> for Language {
    type Error = &'static str;
    fn try_from(name: &str) -> Result<Self, Self::Error> {
        LANGUAGE_INFO.get(name).copied().ok_or("Language not found")
    }
}

/// The set of possible language types
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum LanguageType {
    Data,
    Markup,
    Programming,
    Prose,
}

impl fmt::Display for LanguageType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LanguageType::Data => write!(f, "Data"),
            LanguageType::Markup => write!(f, "Markup"),
            LanguageType::Programming => write!(f, "Programming"),
            LanguageType::Prose => write!(f, "Prose"),
        }
    }
}

/// An enum where the variant is the strategy that detected the language and the value is the name
/// of the language
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Detection {
    Filename(&'static str),
    Extension(&'static str),
    Shebang(&'static str),
    Heuristics(&'static str),
    Classifier(&'static str),
}

impl Detection {
    /// Returns the language detected
    pub fn language(&self) -> &'static str {
        match self {
            Detection::Filename(language)
            | Detection::Extension(language)
            | Detection::Shebang(language)
            | Detection::Heuristics(language)
            | Detection::Classifier(language) => language,
        }
    }

    /// Returns the strategy used to detect the langauge
    pub fn variant(&self) -> &str {
        match self {
            Detection::Filename(_) => "Filename",
            Detection::Extension(_) => "Extension",
            Detection::Shebang(_) => "Shebang",
            Detection::Heuristics(_) => "Heuristics",
            Detection::Classifier(_) => "Classifier",
        }
    }
}

/// Detects the programming language of the file at a given path
///
/// If the language cannot be determined, None will be returned.
/// `detect` will error on an io error or if the parser returns an error when tokenizing the
/// contents of the file
///
/// # Examples
/// ```
/// use std::path::Path;
/// use hyperpolyglot::{detect, Detection};
///
/// let path = Path::new("src/bin/main.rs");
/// let language = detect(path).unwrap().unwrap();
/// assert_eq!(Detection::Heuristics("Rust"), language);
/// ```
pub fn detect(path: &Path) -> Result<Option<Detection>, std::io::Error> {
    let filename = match path.file_name() {
        Some(filename) => filename.to_str(),
        None => return Ok(None),
    };

    let candidate = filename.and_then(|filename| detectors::get_language_from_filename(filename));
    if let Some(candidate) = candidate {
        return Ok(Some(Detection::Filename(candidate)));
    };

    let extension = filename.and_then(|filename| detectors::get_extension(filename));

    let candidates = extension
        .map(|ext| detectors::get_languages_from_extension(ext))
        .unwrap_or_else(Vec::new);

    if candidates.len() == 1 {
        return Ok(Some(Detection::Extension(candidates[0])));
    };

    let file = File::open(path)?;
    let mut reader = BufReader::new(file);

    let candidates = filter_candidates(
        candidates,
        detectors::get_languages_from_shebang(&mut reader)?,
    );
    if candidates.len() == 1 {
        return Ok(Some(Detection::Shebang(candidates[0])));
    };
    reader.seek(SeekFrom::Start(0))?;

    let mut content = String::new();
    reader.read_to_string(&mut content)?;
    let content = truncate_to_char_boundary(&content, MAX_CONTENT_SIZE_BYTES);

    // using heuristics is only going to be useful if we have more than one candidate
    // if the extension didn't result in candidate languages then the heuristics won't either
    let candidates = if candidates.len() > 1 {
        if let Some(extension) = extension {
            let languages =
                detectors::get_languages_from_heuristics(&extension[..], &candidates, &content);
            filter_candidates(candidates, languages)
        } else {
            candidates
        }
    } else {
        candidates
    };

    match candidates.len() {
        0 => Ok(None),
        1 => Ok(Some(Detection::Heuristics(candidates[0]))),
        _ => Ok(Some(Detection::Classifier(detectors::classify(
            &content,
            &candidates,
        )))),
    }
}

// function stolen from from https://doc.rust-lang.org/nightly/src/core/str/mod.rs.html
fn truncate_to_char_boundary(s: &str, mut max: usize) -> &str {
    if max >= s.len() {
        s
    } else {
        while !s.is_char_boundary(max) {
            max -= 1;
        }
        &s[..max]
    }
}

/// Walks the path provided and tallies the programming languages detected in the given path
///
/// Returns a map from the programming languages to a Vec of the files that were detected and the
/// strategy used
///
/// # Examples
/// ```
/// use hyperpolyglot::get_language_breakdown;
/// let breakdown = get_language_breakdown("src/");
/// let total_detections = breakdown.iter().fold(0, |sum, (language, detections)| sum + detections.len());
/// println!("Total files detected: {}", total_detections);
/// ```
pub fn get_language_breakdown<P: AsRef<Path>>(
    path: P,
) -> HashMap<&'static str, Vec<(Detection, PathBuf)>> {
    let override_builder = OverrideBuilder::new(&path);
    let override_builder = filters::add_documentation_override(override_builder);
    let override_builder = filters::add_vendor_override(override_builder);

    let num_threads = env::var_os("HYPLY_THREADS")
        .and_then(|threads| threads.into_string().ok())
        .and_then(|threads| threads.parse().ok())
        .unwrap_or_else(num_cpus::get);

    let (tx, rx) = mpsc::channel::<(Detection, PathBuf)>();
    let walker = WalkBuilder::new(path)
        .threads(num_threads)
        .overrides(override_builder.build().unwrap())
        .build_parallel();

    walker.run(|| {
        let tx = tx.clone();
        Box::new(move |result| {
            use ignore::WalkState::*;

            if let Ok(path) = result {
                let path = path.into_path();
                if !path.is_dir() {
                    if let Ok(Some(detection)) = detect(&path) {
                        tx.send((detection, path)).unwrap();
                    }
                }
            }
            Continue
        })
    });
    drop(tx);

    let mut language_breakdown = HashMap::new();
    for (detection, file) in rx {
        let files = language_breakdown
            .entry(detection.language())
            .or_insert_with(Vec::new);
        files.push((detection, file));
    }

    language_breakdown
}

fn filter_candidates(
    previous_candidates: Vec<&'static str>,
    new_candidates: Vec<&'static str>,
) -> Vec<&'static str> {
    if previous_candidates.is_empty() {
        return new_candidates;
    }

    if new_candidates.is_empty() {
        return previous_candidates;
    }

    let filtered_candidates: Vec<&'static str> = previous_candidates
        .iter()
        .filter(|l| new_candidates.contains(l))
        .copied()
        .collect();

    match filtered_candidates.len() {
        0 => previous_candidates,
        _ => filtered_candidates,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::prelude::*;
    use std::iter;

    #[test]
    fn test_detect_filename() {
        let path = Path::new("APKBUILD");
        let detected_language = detect(path).unwrap().unwrap();

        assert_eq!(detected_language, Detection::Filename("Alpine Abuild"));
    }

    #[test]
    fn test_detect_extension() {
        let path = Path::new("pizza.purs");
        let detected_language = detect(path).unwrap().unwrap();

        assert_eq!(detected_language, Detection::Extension("PureScript"));
    }

    #[test]
    fn test_detect_shebang() {
        let path = Path::new("a");
        let mut file = File::create(path).unwrap();
        file.write(b"#!/usr/bin/python").unwrap();
        file.flush().unwrap();

        let detected_language = detect(path).unwrap().unwrap();

        fs::remove_file(path).unwrap();

        assert_eq!(detected_language, Detection::Shebang("Python"));
    }

    #[test]
    fn test_detect_heuristics() {
        let path = Path::new("a.es");
        let mut file = File::create(path).unwrap();
        file.write(b"'use strict'").unwrap();
        file.flush().unwrap();

        let detected_language = detect(path).unwrap().unwrap();

        fs::remove_file(path).unwrap();

        assert_eq!(detected_language, Detection::Heuristics("JavaScript"));
    }

    #[test]
    fn test_detect_classify() {
        let path = Path::new("peep.rs");
        let mut file = File::create(path).unwrap();
        file.write(
            b"
            match optional {
                Some(pattern) => println!(\"Hello World\"),
                None => println!(\"u missed\")
            }
            ",
        )
        .unwrap();
        file.flush().unwrap();

        let detected_language = detect(path).unwrap().unwrap();

        fs::remove_file(path).unwrap();
        assert_eq!(detected_language, Detection::Classifier("Rust"));
    }

    #[test]
    fn test_detect_none() {
        let path = Path::new("y");
        let mut file = File::create(path).unwrap();
        file.write(
            b"
            use std::io;
            fn main() {
                println!(\"{}\", \"Hello World\");
            }",
        )
        .unwrap();
        file.flush().unwrap();

        let detected_language = detect(path).unwrap();

        fs::remove_file(path).unwrap();

        assert_eq!(detected_language, None);
    }

    #[test]
    fn test_detect_accuracy() {
        let mut total = 0;
        let mut correct = 0;
        fs::read_dir("samples")
            .unwrap()
            .map(|entry| entry.unwrap())
            .filter(|entry| entry.path().is_dir())
            .map(|language_dir| {
                let path = language_dir.path();
                let language = path.file_name().unwrap();
                let language = language.to_string_lossy().into_owned();

                let file_paths = fs::read_dir(language_dir.path())
                    .unwrap()
                    .map(|entry| entry.unwrap().path())
                    .filter(|path| path.is_file());

                let language_iter = iter::repeat(language);
                file_paths.zip(language_iter)
            })
            .flatten()
            .for_each(|(file, language)| {
                // Skip the files we can't detect. The reason the detect function fails on these is
                // because of a heuristic added to .h files that defaults to C if none of the
                // Objective-C or C++ rules match. This makes us fail on two of the sample files
                // but tends to perform better on non training data
                if file.file_name().unwrap() == "rpc.h" || file.file_name().unwrap() == "Field.h" {
                    return;
                }
                // F* uses the name Fstar in the file system
                let language = match &language[..] {
                    "Fstar" => "F*",
                    l => l,
                };
                if let Ok(Some(detection)) = detect(&file) {
                    total += 1;
                    if detection.language() == language {
                        correct += 1;
                    } else {
                        println!("Incorrect detection: {:?} {:?}", file, detection)
                    }
                }
            });

        let accuracy = (correct as f64) / (total as f64);
        assert_eq!(accuracy, 1.0);
    }

    #[test]
    fn test_filter_candidates() {
        let previous_candidates = vec!["JavaScript", "Python"];
        let new_candidates = vec!["Python", "Bibbity"];
        assert_eq!(
            filter_candidates(previous_candidates, new_candidates),
            vec!["Python"]
        );
    }

    #[test]
    fn test_filter_candidates_no_new() {
        let previous_candidates = vec!["JavaScript", "Python"];
        let new_candidates = vec![];
        assert_eq!(
            filter_candidates(previous_candidates, new_candidates),
            vec!["JavaScript", "Python"]
        );
    }

    #[test]
    fn test_filter_candidates_no_prev() {
        let previous_candidates = vec![];
        let new_candidates = vec!["JavaScript", "Erlang"];
        assert_eq!(
            filter_candidates(previous_candidates, new_candidates),
            vec!["JavaScript", "Erlang"]
        );
    }

    #[test]
    fn test_filter_candidates_no_matches() {
        let previous_candidates = vec!["Python"];
        let new_candidates = vec!["JavaScript", "Erlang"];
        assert_eq!(
            filter_candidates(previous_candidates, new_candidates),
            vec!["Python"]
        );
    }

    #[test]
    fn test_get_language_breakdown_ignores_overrides_documentation() {
        fs::create_dir_all("temp-testing-dir").unwrap();
        fs::File::create("temp-testing-dir/README.md").unwrap();
        assert!(get_language_breakdown("temp-testing-dir").is_empty());

        fs::remove_dir_all("temp-testing-dir").unwrap();
    }

    #[test]
    fn test_get_language_breakdown_ignores_overrides_vendor() {
        fs::create_dir_all("temp-testing-dir2/node_modules").unwrap();
        fs::File::create("temp-testing-dir2/node_modules/hello.go").unwrap();
        assert!(get_language_breakdown("temp-testing-dir2").is_empty());

        fs::remove_dir_all("temp-testing-dir2").unwrap();
    }
}
