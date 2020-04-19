//! # Hyperpolyglot
//! `hyperpolyglot` is a crate for detecting the programming language of a file or the language
//! breakdown for a directory.
#![feature(test)]

use ignore::{overrides::OverrideBuilder, WalkBuilder};
use std::{
    collections::HashMap,
    error::Error,
    fmt,
    fs::File,
    io::{BufReader, Read, Seek, SeekFrom},
    path::{Path, PathBuf},
    sync::mpsc,
};

mod classifier;
mod documentation;
mod extension;
mod filenames;
mod heuristics;
mod interpreter;
pub mod tokenizer;
mod vendor;

// Include the map that stores language info
// static LANGUAGE_INFO: phf::Map<&'static str, Language> = ...;
include!("codegen/language-info-map.rs");

const MAX_CONTENT_SIZE_BYTES: usize = 51200;

/// The language object that conatins the name and the type of language
#[derive(Debug)]
pub struct Language<'a> {
    pub name: &'a str,
    pub language_type: LanguageType,
}

/// The set of possible language types
#[derive(Debug)]
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

/// An enum where the variant is the streategy that detected the language and the value is the name
/// of the language
#[derive(Debug, PartialEq)]
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
    pub fn variant(&self) -> String {
        match self {
            Detection::Filename(_) => String::from("Filename"),
            Detection::Extension(_) => String::from("Extension"),
            Detection::Shebang(_) => String::from("Shebang"),
            Detection::Heuristics(_) => String::from("Heuristics"),
            Detection::Classifier(_) => String::from("Classifier"),
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
/// let path = Path::new("src/main.rs");
/// let language = detect(path).unwrap().unwrap();
/// assert_eq!(Detection::Heuristics("Rust"), language);
/// ```
pub fn detect(path: &Path) -> Result<Option<Detection>, Box<dyn Error>> {
    let filename = path.file_name().and_then(|filename| filename.to_str());

    let candidate = filename.and_then(|filename| filenames::get_language_from_filename(filename));
    if let Some(candidate) = candidate {
        return Ok(Some(Detection::Filename(candidate)));
    };

    let extension = filename.and_then(|filename| extension::get(filename));

    let candidates = extension
        .map(|ext| extension::get_languages_from_extension(ext))
        .unwrap_or(vec![]);

    if candidates.len() == 1 {
        return Ok(Some(Detection::Extension(candidates[0])));
    };

    let file = File::open(path)?;
    let mut reader = BufReader::new(file);

    let candidates = filter_candidates(
        candidates,
        interpreter::get_languages_from_shebang(&mut reader)?,
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
                heuristics::get_languages_from_heuristics(&extension[..], &candidates, &content);
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
        _ => Ok(Some(Detection::Classifier(classifier::classify(
            &content,
            &candidates,
        )?))),
    }
}

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
/// let breakdown = hyperpolyglot::get_language_breakdown("src/");
/// println!("{:?}", breakdown.get("Rust"));
/// ```
pub fn get_language_breakdown<P: AsRef<Path>>(
    path: P,
) -> HashMap<&'static str, Vec<(Detection, PathBuf)>> {
    let override_builder = OverrideBuilder::new(&path);
    let override_builder = documentation::add_override(override_builder);
    let override_builder = vendor::add_override(override_builder);

    let (tx, rx) = mpsc::channel::<(Detection, PathBuf)>();
    let walker = WalkBuilder::new(path)
        .overrides(override_builder.build().unwrap())
        .build_parallel();
    walker.run(|| {
        let tx = tx.clone();
        Box::new(move |result| {
            use ignore::WalkState::*;

            let path = result.unwrap().into_path();
            if let Ok(Some(detection)) = detect(&path) {
                tx.send((detection, path)).unwrap();
            }
            Continue
        })
    });
    drop(tx);

    let mut language_breakdown = HashMap::new();
    for (detection, file) in rx {
        let files = language_breakdown
            .entry(detection.language())
            .or_insert(vec![]);
        files.push((detection, file));
    }

    language_breakdown
}

fn filter_candidates(
    previous_candidates: Vec<&'static str>,
    new_candidates: Vec<&'static str>,
) -> Vec<&'static str> {
    if previous_candidates.len() == 0 {
        return new_candidates;
    }

    if new_candidates.len() == 0 {
        return previous_candidates;
    }

    let filtered_candidates: Vec<&'static str> = previous_candidates
        .iter()
        .filter(|l| new_candidates.contains(l))
        .map(|l| *l)
        .collect();

    match filtered_candidates.len() {
        0 => previous_candidates,
        _ => filtered_candidates,
    }
}

/// Returns the info about a language given a language name
///
/// If the function is called with a language returned from `detect` or `get_language_breakdown`
/// then the value can be unwrapped because it is guaranteed to be there
///
/// # Examples
/// ```
/// let info = hyperpolyglot::get_language_info("Rust").unwrap();
/// assert_eq!(info.language_type.to_string(), "Programming")
/// ```
pub fn get_language_info(name: &str) -> Option<&Language> {
    LANGUAGE_INFO.get(name)
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
                // F* uses the name Fstar in the file system
                let language = match &language[..] {
                    "Fstar" => "F*",
                    l => l,
                };
                if let Ok(Some(detection)) = detect(&file) {
                    total += 1;
                    if detection.language() == language {
                        correct += 1;
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
