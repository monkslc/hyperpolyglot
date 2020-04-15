use pcre2::bytes::RegexBuilder as PCRERegex;

// Include the map from interpreters to languages at compile time
// static DISAMBIGUATIONS: phf::Map<&'static str, &'static [Rule]> = ...;
include!("codegen/disambiguation-heuristics-map.rs");

#[derive(Debug)]
enum Pattern {
    And(&'static [Pattern]),
    Negative(&'static str),
    Or(&'static [Pattern]),
    Positive(&'static str),
}

#[derive(Debug)]
struct Rule {
    languages: &'static [&'static str],
    pattern: Option<Pattern>,
}

impl Pattern {
    fn matches(&self, content: &str) -> bool {
        match self {
            Pattern::Positive(pattern) => {
                let regex = PCRERegex::new()
                    .crlf(true)
                    .multi_line(true)
                    .build(pattern)
                    .unwrap();
                regex.is_match(content.as_bytes()).unwrap_or(false)
            }
            Pattern::Negative(pattern) => {
                let regex = PCRERegex::new()
                    .crlf(true)
                    .multi_line(true)
                    .build(pattern)
                    .unwrap();
                !regex.is_match(content.as_bytes()).unwrap_or(true)
            }
            Pattern::Or(patterns) => patterns.iter().any(|pattern| pattern.matches(content)),
            Pattern::And(patterns) => patterns.iter().all(|pattern| pattern.matches(content)),
        }
    }
}

pub fn get_languages_from_heuristics(
    extension: &str,
    candidates: &Vec<&'static str>,
    content: &str,
) -> Vec<&'static str> {
    match DISAMBIGUATIONS.get(extension) {
        Some(rules) => {
            let rules = rules.iter().filter(|rule| {
                rule.languages
                    .iter()
                    .all(|language| candidates.contains(language))
            });
            for rule in rules {
                if let Some(pattern) = &rule.pattern {
                    if pattern.matches(content) {
                        return rule.languages.to_vec();
                    };
                } else {
                    // if there is no pattern then it is a match by default
                    return rule.languages.to_vec();
                };
            }
            vec![]
        }
        None => vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_heuristics_get_languages_positive_pattern() {
        assert_eq!(
            get_languages_from_heuristics(".es", &vec!["Erlang", "JavaScript"], "'use strict';"),
            vec!["JavaScript"]
        );
    }

    #[test]
    fn test_heuristics_get_languages_negative_pattern() {
        assert_eq!(
            get_languages_from_heuristics(
                ".sql",
                &vec!["PLSQL", "PLpgSQL", "SQL", "SQLPL", "TSQL"],
                "LALA THIS IS SQL"
            ),
            vec!["SQL"]
        );
    }

    #[test]
    fn test_heuristics_get_languages_and_positives_pattern() {
        assert_eq!(
            get_languages_from_heuristics(
                ".pro",
                &vec!["Proguard", "Prolog", "INI", "QMake", "IDL"],
                "HEADERS SOURCES"
            ),
            vec!["QMake"]
        );
    }

    #[test]
    fn test_heuristics_get_languages_and_not_all_match() {
        let empty_vec: Vec<&'static str> = vec![];
        assert_eq!(
            get_languages_from_heuristics(
                ".pro",
                &vec!["Proguard", "Prolog", "INI", "QMake", "IDL"],
                "HEADERS"
            ),
            empty_vec
        );
    }

    #[test]
    fn test_heuristics_get_languages_and_negative_pattern() {
        assert_eq!(
            get_languages_from_heuristics(
                ".ms",
                &vec!["Roff", "Unix Assembly", "MAXScript"],
                ".include:"
            ),
            vec!["Unix Assembly"]
        );
    }

    #[test]
    fn test_heuristics_get_languages_or_pattern() {
        assert_eq!(
            get_languages_from_heuristics(".p", &vec!["Gnuplot", "OpenEdge ABL"], "plot"),
            vec!["Gnuplot"]
        );
    }

    #[test]
    fn test_heuristics_get_languages_named_pattern() {
        assert_eq!(
            get_languages_from_heuristics(".h", &vec!["Objective-C", "C++"], "std::out"),
            vec!["C++"]
        );
    }

    #[test]
    fn test_heuristics_get_languages_default_pattern() {
        assert_eq!(
            get_languages_from_heuristics(".man", &vec!["Roff Manpage", "Roff"], "alskdjfahij"),
            vec!["Roff"]
        );
    }

    #[test]
    fn test_heuristics_get_languages_multiple_anchors() {
        assert_eq!(
            get_languages_from_heuristics(
                ".1in",
                &vec!["Roff Manpage", "Roff"],
                r#".TH LYXCLIENT 1 "@LYX_DATE@" "Version @VERSION@" "lyxclient @VERSION@"
.SH NAME"#
            ),
            vec!["Roff Manpage"]
        );
    }
}
