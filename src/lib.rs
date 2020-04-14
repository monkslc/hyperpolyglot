#![feature(test)]

use ignore::{overrides::OverrideBuilder, WalkBuilder};
use std::{
    collections::HashMap,
    error::Error,
    fs::File,
    io::{BufReader, Read, Seek, SeekFrom},
    path::Path,
};

mod classifier;
mod documentation;
mod extension;
mod filenames;
mod heuristics;
mod interpreter;
mod vendor;

// Include the map that stores language info
// static LANGUAGE_INFO: phf::Map<&'static str, Language> = ...;
include!("codegen/language-info-map.rs");

pub struct Language<'a> {
    pub name: &'a str,
    pub type_of: LanguageType,
}

pub enum LanguageType {
    Data,
    Markup,
    Programming,
    Prose,
}

pub fn detect(path: &Path) -> Result<&'static str, Box<dyn Error>> {
    let filename = path.file_name().and_then(|filename| filename.to_str());

    let candidate = filename.and_then(|filename| filenames::get_language_by_filename(filename));
    if let Some(candidate) = candidate {
        return Ok(candidate);
    };

    let extension = filename.and_then(|filename| extension::get(filename));

    let candidates = extension
        .map(|ext| extension::get_language(ext))
        .unwrap_or(vec![]);

    if candidates.len() == 1 {
        return Ok(candidates[0]);
    };

    let file = File::open(path)?;
    let mut reader = BufReader::new(file);

    let candidates = filter_candidates(
        candidates,
        interpreter::get_language_by_shebang(&mut reader)?,
    );
    if candidates.len() == 1 {
        return Ok(candidates[0]);
    };
    reader.seek(SeekFrom::Start(0))?;

    let mut content = String::new();
    reader.read_to_string(&mut content)?;

    // using heuristics is only going to be useful if we have more than one candidate
    // if the extension didn't result in candidate languages then the heuristics won't either
    let candidates = if candidates.len() > 1 {
        if let Some(extension) = extension {
            let languages = heuristics::get_languages(&extension[..], &candidates, &content);
            filter_candidates(candidates, languages)
        } else {
            candidates
        }
    } else {
        candidates
    };

    if candidates.len() == 1 {
        return Ok(candidates[0]);
    }

    classifier::classify(&content, &candidates)
}

pub fn get_language_breakdown<P: AsRef<Path>>(path: P) -> HashMap<&'static str, i32> {
    let mut counts = HashMap::new();
    let override_builder = OverrideBuilder::new(&path);
    let override_builder = documentation::add_override(override_builder);
    let override_builder = vendor::add_override(override_builder);
    WalkBuilder::new(&path)
        .overrides(override_builder.build().unwrap())
        .build()
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry
                .file_type()
                .map(|file| file.is_file())
                .unwrap_or(false)
        })
        .for_each(|entry| {
            if let Ok(language) = detect(entry.path()) {
                let count = counts.entry(language).or_insert(0);
                *count += 1;
            }
        });
    counts
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
        let detected_language = detect(path).unwrap();

        assert_eq!(detected_language, "Alpine Abuild");
    }

    #[test]
    fn test_detect_extension() {
        let path = Path::new("pizza.purs");
        let detected_language = detect(path).unwrap();

        assert_eq!(detected_language, "PureScript");
    }

    #[test]
    fn test_detect_shebang() {
        let path = Path::new("a");
        let mut file = File::create(path).unwrap();
        file.write(b"#!/usr/bin/python").unwrap();
        file.flush().unwrap();

        let detected_language = detect(path).unwrap();

        fs::remove_file(path).unwrap();

        assert_eq!(detected_language, "Python");
    }

    #[test]
    fn test_detect_heuristics() {
        let path = Path::new("a.es");
        let mut file = File::create(path).unwrap();
        file.write(b"'use strict'").unwrap();
        file.flush().unwrap();

        let detected_language = detect(path).unwrap();

        fs::remove_file(path).unwrap();

        assert_eq!(detected_language, "JavaScript");
    }

    #[test]
    fn test_detect_classify() {
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

        assert_eq!(detected_language, "Rust");
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
                if let Ok(detected_language) = detect(&file) {
                    total += 1;
                    if detected_language == language {
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
