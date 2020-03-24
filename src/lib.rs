use lazy_static::lazy_static;
use pcre2::bytes::Regex as PCRERegex;
use regex::Regex;
use std::io::{self, BufRead, BufReader, Read};

// Include the map from filenames to languages at compile time
// static FILENAMES: phf::Map<&'static str, &'static str> = ...;
include!(concat!(env!("OUT_DIR"), "/filename-language-map.rs"));

// Include the map from interpreters to languages at compile time
// static INTERPRETERS: phf::Map<&'static str, &[&str]> = ...;
include!(concat!(env!("OUT_DIR"), "/interpreter-language-map.rs"));

// Include the map from interpreters to languages at compile time
// static EXTENSIONS: phf::Map<&'static str, &[&str]> = ...;
include!(concat!(env!("OUT_DIR"), "/extension-language-map.rs"));

// Include the map from interpreters to languages at compile time
// static DISAMBIGUATIONS: phf::Map<&'static str, Rule> = ...;
include!(concat!(
    env!("OUT_DIR"),
    "/disambiguation-heuristics-map.rs"
));

#[derive(Debug)]
enum Pattern {
    And(&'static [Pattern]),
    Negative(&'static str),
    Or(&'static [Pattern]),
    Positive(&'static str),
}

#[derive(Debug)]
struct Rule {
    language: &'static str,
    pattern: Option<Pattern>,
}

impl Pattern {
    fn matches(&self, content: &str) -> bool {
        match self {
            Pattern::Positive(pattern) => {
                let regex = PCRERegex::new(pattern).unwrap();
                regex.is_match(content.as_bytes()).unwrap_or(false)
            }
            Pattern::Negative(pattern) => {
                let regex = PCRERegex::new(pattern).unwrap();
                !regex.is_match(content.as_bytes()).unwrap_or(true)
            }
            Pattern::Or(patterns) => patterns.iter().any(|pattern| pattern.matches(content)),
            Pattern::And(patterns) => patterns.iter().all(|pattern| pattern.matches(content)),
        }
    }
}

pub fn get_language_by_filename(filename: &str) -> Option<&&'static str> {
    FILENAMES.get(filename)
}

pub fn get_language_by_shebang<R: Read>(reader: R) -> Result<Vec<&'static str>, io::Error> {
    let mut reader = BufReader::new(reader);
    let mut line = String::new();
    reader.read_line(&mut line)?;
    if !line.starts_with("#!") {
        return Ok(vec![]);
    }

    let languages = line
        .split("/")
        .last()
        .and_then(|interpreter_line| {
            let mut splits = interpreter_line.split_whitespace();
            match splits.next() {
                // #!/usr/bin/env python
                Some("env") => splits.next(),
                // #!/usr/bin/python
                Some(interpreter) => Some(interpreter),
                // #!
                None => None,
            }
        })
        .and_then(|interpreter| {
            // #!/usr/bin/python2.6.3 -> #!/usr/bin/python2
            lazy_static! {
                static ref RE: Regex = Regex::new(r#"[0-9]\.[0-9]"#).unwrap();
            }
            let interpreter = RE.split(interpreter).next().unwrap();

            INTERPRETERS.get(interpreter)
        });

    match languages {
        Some(languages) => Ok(languages.to_vec()),
        None => Ok(vec![]),
    }
}

pub fn get_language_by_extension(filename: &str) -> Vec<&'static str> {
    let extension = get_extension(filename);
    let extension = extension.as_str();
    let languages = EXTENSIONS
        .get(extension)
        .map(|languages| languages.to_vec());

    match languages {
        Some(languages) => languages,
        None => vec![],
    }
}

pub fn disambiguate_extension_overlap(
    extension: &str,
    candidates: &Vec<&'static str>,
    content: &str,
) -> Option<&'static str> {
    match DISAMBIGUATIONS.get(extension) {
        Some(rules) => {
            for rule in rules.iter() {
                if candidates.contains(&rule.language) {
                    if let Some(pattern) = &rule.pattern {
                        if pattern.matches(content) {
                            return Some(rule.language);
                        };
                    } else {
                        // if there is not a pattern its a match by default
                        return Some(rule.language);
                    };
                };
            }
            None
        }
        None => None,
    }
}

fn get_extension(filename: &str) -> String {
    let filename = if filename.starts_with(".") {
        &filename[1..]
    } else {
        filename
    };

    let extension_parts: Vec<&str> = filename.split(".").skip(1).collect();
    extension_parts.join(".")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_get_language_by_filename() {
        assert_eq!(get_language_by_filename("APKBUILD"), Some(&"Alpine Abuild"));
        assert_eq!(
            get_language_by_filename(".eslintrc.json"),
            Some(&"JSON with Comments")
        );
    }

    #[test]
    fn test_get_language_by_shebang() {
        assert_eq!(
            get_language_by_shebang(Cursor::new("#!/usr/bin/python")).unwrap(),
            vec!["Python"]
        );

        assert_eq!(
            get_language_by_shebang(Cursor::new("#!/usr/bin/env node")).unwrap(),
            vec!["JavaScript"]
        );

        let mut parrot_langs = get_language_by_shebang(Cursor::new("#!/usr/bin/parrot")).unwrap();
        parrot_langs.sort();
        assert_eq!(
            parrot_langs,
            vec!["Parrot Assembly", "Parrot Internal Representation"]
        );

        assert_eq!(
            get_language_by_shebang(Cursor::new("#!/usr/bin/python2.6")).unwrap(),
            vec!["Python"]
        );

        let empty_vec: Vec<&'static str> = Vec::new();
        assert_eq!(
            get_language_by_shebang(Cursor::new("#!/usr/bin/env")).unwrap(),
            empty_vec
        );

        assert_eq!(
            get_language_by_shebang(Cursor::new("#!")).unwrap(),
            empty_vec
        );

        assert_eq!(get_language_by_shebang(Cursor::new("")).unwrap(), empty_vec);
        assert_eq!(
            get_language_by_shebang(Cursor::new("aslkdfjas;ldk")).unwrap(),
            empty_vec
        );

        assert_eq!(
            get_language_by_shebang(Cursor::new(" #!/usr/bin/python")).unwrap(),
            empty_vec
        );

        assert_eq!(
            get_language_by_shebang(Cursor::new(" #!/usr/bin/ ")).unwrap(),
            empty_vec
        );

        assert_eq!(
            get_language_by_shebang(Cursor::new(" #!/usr/bin")).unwrap(),
            empty_vec
        );
    }

    #[test]
    fn test_get_language_by_extension() {
        assert_eq!(get_language_by_extension("index.djs"), vec!["Dogescript"]);
        assert_eq!(get_language_by_extension("example.cmake.in"), vec!["CMake"]);

        let mut header_file_langs = get_language_by_extension("level.h");
        header_file_langs.sort();
        assert_eq!(header_file_langs, vec!["C", "C++", "Objective-C"]);

        let empty_vec: Vec<&'static str> = vec![];
        assert_eq!(get_language_by_extension("hello.kasdjf"), empty_vec);

        assert_eq!(get_language_by_extension(".c"), empty_vec);
        assert_eq!(get_language_by_extension(""), empty_vec);
        assert_eq!(get_language_by_extension("noextension"), empty_vec);
    }

    #[test]
    fn test_disambiguate_extension_overlap() {
        // Matches Positive Pattern
        assert_eq!(
            disambiguate_extension_overlap("es", &vec!["Erlang", "JavaScript"], "'use strict';"),
            Some("JavaScript")
        );

        // Matches Negative Pattern
        assert_eq!(
            disambiguate_extension_overlap(
                "sql",
                &vec!["PLSQL", "PLpgSQL", "SQL", "SQLPL", "TSQL"],
                "LALA THIS IS SQL"
            ),
            Some("SQL")
        );

        // Matches And with all positives
        assert_eq!(
            disambiguate_extension_overlap(
                "pro",
                &vec!["Proguard", "Prolog", "INI", "QMake", "IDL"],
                "HEADERS SOURCES"
            ),
            Some("QMake")
        );

        // Doesn't match And if less than all match
        assert_eq!(
            disambiguate_extension_overlap(
                "pro",
                &vec!["Proguard", "Prolog", "INI", "QMake", "IDL"],
                "HEADERS"
            ),
            None
        );

        // Matches And with negative pattern
        assert_eq!(
            disambiguate_extension_overlap(
                "ms",
                &vec!["Roff", "Unix Assembly", "MAXScript"],
                ".include:"
            ),
            Some("Unix Assembly")
        );

        // Matches Or if one is true
        assert_eq!(
            disambiguate_extension_overlap("p", &vec!["Gnuplot", "OpenEdge ABL"], "plot"),
            Some("Gnuplot")
        );

        // Matches named pattern
        assert_eq!(
            disambiguate_extension_overlap("h", &vec!["Objective-C", "C++"], "std::out"),
            Some("C++")
        );

        // Matches default language pattern (no pattern specified)
        assert_eq!(
            disambiguate_extension_overlap("man", &vec!["Roff Manpage", "Roff"], "alskdjfahij"),
            Some("Roff")
        );
    }
}
