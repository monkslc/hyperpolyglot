use std::error::Error;

// Include the map that counts token occurences per language
// static TOKEN_COUNTS: phf::Map<&'static str, phf::Map<&'static str, f64>> = ...;
include!("codegen/token-count.rs");

// Include the map that counts the total number of tokens for a language
// static TOTAL_TOKEN_COUNT: phf::Map<&'static str, f64> = ...;
include!("codegen/total-token-count.rs");

// Include the array of all possible languages
// static LANGUAGES: &[&'static str] = ...;
include!("codegen/languages.rs");

const MAX_TOKEN_BYTES: usize = 32;

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
            let score = match TOKEN_COUNTS.get(language) {
                Some(token_map) => {
                    // unwrap is safe here because the entry will be there if there was an entry in
                    // TOKEN_COUNTS for that language
                    let total_tokens = *TOTAL_TOKEN_COUNT.get(language).unwrap();
                    tokens
                        .iter()
                        .filter(|token| token.len() <= MAX_TOKEN_BYTES)
                        .fold(0f64, |sum, token| {
                            let token_prob = token_probability(token_map, token, total_tokens).ln();
                            sum + token_prob
                        })
                }
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

fn token_probability(token_map: &phf::Map<&'static str, f64>, token: &str, total: f64) -> f64 {
    let count = token_map.get(token).unwrap_or(&1E-5f64);
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

    #[test]
    fn test_classify_f_star() {
        let content = fs::read_to_string("samples/Fstar/Hacl.HKDF.fst").unwrap();
        let candidates = vec![];
        let language = classify(content.as_str(), &candidates).unwrap();
        assert_eq!(language, "F*");
    }

    #[bench]
    fn bench_token_probability(b: &mut Bencher) {
        let token_map_rust = TOKEN_COUNTS.get("Rust").unwrap();
        let token_map_jup = TOKEN_COUNTS.get("Jupyter Notebook").unwrap();
        let token_map_objc = TOKEN_COUNTS.get("Objective-C").unwrap();
        let token_map_ts = TOKEN_COUNTS.get("TypeScript").unwrap();

        let tokens_rust = *TOTAL_TOKEN_COUNT.get("Rust").unwrap();
        let tokens_jup = *TOTAL_TOKEN_COUNT.get("Jupyter Notebook").unwrap();
        let tokens_objc = *TOTAL_TOKEN_COUNT.get("Objective-C").unwrap();
        let tokens_ts = *TOTAL_TOKEN_COUNT.get("TypeScript").unwrap();
        b.iter(|| {
            token_probability(token_map_rust, "fn", tokens_rust);
            token_probability(token_map_jup, "kSEFGUQI3rHsywBz1dB", tokens_jup);
            token_probability(token_map_objc, "setDefaultCredential", tokens_objc);
            token_probability(token_map_ts, "Not actually there990", tokens_ts);
        });
    }

    #[bench]
    #[ignore] // too expensive
    fn bench_classify_long(b: &mut Bencher) {
        let content = fs::read_to_string("samples/Rust/hashmap.rs").unwrap();
        let content = &content[..];
        b.iter(|| {
            let _ = classify(content, &vec![]);
        });
    }

    #[bench]
    fn bench_classify_short(b: &mut Bencher) {
        b.iter(|| {
            let _ = classify("fn main() {}", &vec![]);
        });
    }
}
