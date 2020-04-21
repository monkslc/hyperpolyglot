use std::error::Error;

use crate::tokenizer;

// Include the map that contains the token log probabilities
// static TOKEN_LOG_PROBABILITIES: phf::Map<&'static str, f64> = ...;
include!("../codegen/token-log-probabilities.rs");

// Include the array of all possible languages
// static LANGUAGES: &[&'static str] = ...;
include!("../codegen/languages.rs");

const MAX_TOKEN_BYTES: usize = 32;
const DEFAULT_LOG_PROB: f64 = -19f64;

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

    let mut scored_candidates: Vec<LanguageScore> = candidates
        .iter()
        .map(|language| {
            let score = match TOKEN_LOG_PROBABILITIES.get(language) {
                Some(token_map) => tokenizer::tokenize(content)
                    .filter(|token| token.len() <= MAX_TOKEN_BYTES)
                    .fold(0f64, |sum, token| {
                        let token_log_prob = token_map.get(token).unwrap_or(&DEFAULT_LOG_PROB);
                        sum + token_log_prob
                    }),
                None => std::f64::NEG_INFINITY,
            };
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
        let candidates = vec!["Rust", "RenderScript"];
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

    #[test]
    fn test_classify_f_star() {
        let content = fs::read_to_string("samples/Fstar/Hacl.HKDF.fst").unwrap();
        let candidates = vec![];
        let language = classify(content.as_str(), &candidates).unwrap();
        assert_eq!(language, "F*");
    }

    #[bench]
    fn bench_classify_short(b: &mut Bencher) {
        b.iter(|| {
            let _ = classify("fn main() {}", &vec![]);
        });
    }
}
