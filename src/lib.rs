use ignore::Walk;
use std::{
    collections::HashMap,
    error::Error,
    fs::File,
    io::{BufRead, BufReader, Read, Seek, SeekFrom},
    path::Path,
};

mod classifier;
mod extension;
mod filenames;
mod heuristics;
mod interpreter;

pub fn detect(path: &Path) -> Result<&'static str, Box<dyn Error>> {
    let filename = path.file_name().and_then(|filename| filename.to_str());

    let candidate = filename.and_then(|filename| filenames::get_language_by_filename(filename));
    if let Some(candidate) = candidate {
        return Ok(candidate);
    };

    let extension = filename.and_then(|filename| get_extension(filename));

    let candidates = extension
        .map(|ext| extension::get_language_by_extension(ext))
        .unwrap_or(vec![]);

    if candidates.len() == 1 {
        return Ok(candidates[0]);
    };

    let file = File::open(path)?;
    let mut reader = BufReader::new(file);

    let mut line = String::new();
    reader.read_line(&mut line)?;

    let candidates = filter_candidates(candidates, interpreter::get_language_by_shebang(&line[..]));
    if candidates.len() == 1 {
        return Ok(candidates[0]);
    };
    reader.seek(SeekFrom::Start(0))?;

    let mut content = String::new();
    reader.read_to_string(&mut content)?;

    if candidates.len() > 1 {
        let language = extension
            .and_then(|ext| heuristics::disambiguate_overlap(&ext[..], &candidates, &content));
        if let Some(language) = language {
            return Ok(language);
        };
    }

    classifier::classify(&content, &candidates)
}

pub fn get_language_breakdown() -> HashMap<&'static str, i32> {
    let mut counts = HashMap::new();
    Walk::new("./")
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

fn get_extension(filename: &str) -> Option<&str> {
    let filename = if filename.starts_with(".") {
        &filename[1..]
    } else {
        filename
    };

    filename.find(".").map(|i| &filename[i + 1..])
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::prelude::*;

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
    fn test_get_extension() {
        assert_eq!(get_extension("index.djs"), Some("djs"));
        assert_eq!(get_extension("example.cmake.in"), Some("cmake.in"));
        assert_eq!(get_extension(".eslintrc.json"), Some("json"));
        assert_eq!(get_extension(".cs"), None);
        assert_eq!(get_extension("noextension"), None);
        // Following tests are to test behavior that may break
        // and are not expected funcitonality
        assert_eq!(get_extension(".es."), Some(""));
        assert_eq!(get_extension(".."), Some(""));
    }
}
