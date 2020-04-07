use std::error::Error;

// Include the map that counts token occurences per language
// static TOKEN_COUNTS: phf::Map<&'static str, f64> = ...;
include!("codegen/token-count.rs");

// Include the map that counts the total number of tokens for a language
// static TOTAL_TOKEN_COUNT: phf::Map<&'static str, f64> = ...;
include!("codegen/total-token-count.rs");

// Include the array of all possible languages
// static LANGUAGES: &[&'static str] = ...;
include!("codegen/languages.rs");

#[derive(Debug)]
pub struct LanguageScore {
    language: &'static str,
    score: f64,
}

pub fn classify(
    content: &str,
    candidates: &Vec<&'static str>,
) -> Result<&'static str, Box<dyn Error>> {
    let candidates = match candidates.len() {
        0 => LANGUAGES,
        _ => candidates,
    };

    let tokens = tokens::tokenize(content)?;
    let mut scored_candidates: Vec<LanguageScore> = candidates
        .iter()
        .map(|language| {
            let score = tokens.iter().fold(0f64, |sum, token| {
                let token_prob = token_probability(language, token).ln();
                sum + token_prob
            });
            LanguageScore {
                language: language,
                score,
            }
        })
        .collect();

    scored_candidates.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    Ok(scored_candidates[0].language)
}

fn token_probability(language: &str, token: &str) -> f64 {
    let count = match TOKEN_COUNTS.get(language) {
        Some(map) => map.get(token).unwrap_or(&1E-5f64),
        None => &1E-5f64,
    };

    // Can't just unwrap here because there are languages in the languages.yml
    // file that we don't have samples for and therefore no tokens have been seen
    let total = TOTAL_TOKEN_COUNT.get(language).unwrap_or(&1f64);
    count / total
}

#[cfg(test)]
mod tests {
    extern crate test;
    use super::*;
    use std::fs;
    use test::Bencher;

    #[test]
    fn test_classify() {
        let content = fs::read_to_string("samples/Rust/main.rs").unwrap();
        let candidates = vec!["C", "Rust"];
        let language = classify(content.as_str(), &candidates).unwrap();
        assert_eq!(language, "Rust");

        let content = fs::read_to_string("samples/Erlang/170-os-daemons.es").unwrap();
        let candidates = vec!["Erlang", "JavaScript"];
        let language = classify(content.as_str(), &candidates).unwrap();
        assert_eq!(language, "Erlang");

        let content = fs::read_to_string("samples/TypeScript/classes.ts").unwrap();
        let candidates = vec!["C++", "Java", "C#", "TypeScript"];
        let language = classify(content.as_str(), &candidates).unwrap();
        assert_eq!(language, "TypeScript");
    }

    #[test]
    fn test_classify_non_sample_data() {
        let sample = r#"#[cfg(not(feature = "pcre2"))]
    fn imp(args: &Args) -> Result<bool> {
        let mut stdout = args.stdout();
        writeln!(stdout, "PCRE2 is not available in this build of ripgrep.")?;
        Ok(false)
    }

    imp(args)"#;
        let candidates = vec!["Rust", "C", "C++"];
        let language = classify(sample, &candidates).unwrap();
        assert_eq!(language, "Rust");
    }

    #[test]
    fn test_classify_empty_candidates() {
        let content = fs::read_to_string("samples/Rust/main.rs").unwrap();
        let candidates = vec![];
        let language = classify(content.as_str(), &candidates).unwrap();
        assert_eq!(language, "Rust");
    }

    #[bench]
    fn bench_token_probability(b: &mut Bencher) {
        b.iter(|| {
            token_probability("Rust", "fn");
            token_probability("Jupyter Notebook", "kSEFGUQI3rHsywBz1dB");
            token_probability("Objective-C", "setDefaultCredential");
            token_probability("TypeScript", "Not actually there990");
            token_probability("Not realassdf", "struct");
        });
    }
}
