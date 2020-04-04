use pcre2::bytes::Regex as PCRERegex;

// Include the map from interpreters to languages at compile time
// static DISAMBIGUATIONS: phf::Map<&'static str, Rule> = ...;
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

#[cfg(test)]
mod tests {
    use super::*;

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
