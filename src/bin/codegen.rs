use pcre2::bytes::Regex as PCRERegex;
use phf_codegen::Map as PhfMap;
use serde::Deserialize;
use std::{
    collections::HashMap,
    fs::{self, File},
    io::{BufWriter, Write},
    iter,
};

type LanguageMap = HashMap<String, LanguageDTO>;
type NamedPatterns = HashMap<String, MaybeMany<String>>;

#[derive(Deserialize)]
struct LanguageDTO {
    filenames: Option<Vec<String>>,
    interpreters: Option<Vec<String>>,
    extensions: Option<Vec<String>>,
    #[serde(rename(deserialize = "type"))]
    language_type: LanguageType,
    color: Option<String>,
    group: Option<String>,
}

impl LanguageDTO {
    fn to_domain_object_code(&self, name: &str) -> String {
        format!(
            "Language {{ name: \"{}\", language_type: {}, color: {:?}, group: {:?} }}",
            name,
            self.language_type.to_domain_object_code(),
            self.color,
            self.group
        )
    }
}

#[derive(Deserialize, Debug)]
enum LanguageType {
    #[serde(rename = "data")]
    Data,
    #[serde(rename = "markup")]
    Markup,
    #[serde(rename = "programming")]
    Programming,
    #[serde(rename = "prose")]
    Prose,
}

impl LanguageType {
    fn to_domain_object_code(&self) -> String {
        format!("LanguageType::{:?}", self)
    }
}

#[derive(Deserialize)]
struct Heuristics {
    disambiguations: Vec<Disambiguation>,
    named_patterns: NamedPatterns,
}

#[derive(Deserialize)]
struct Disambiguation {
    extensions: Vec<String>,
    rules: Vec<RuleDTO>,
}

impl Disambiguation {
    fn to_domain_object_code(&self, named_patterns: &NamedPatterns) -> String {
        let mut rules = String::new();
        for rule in self.rules.iter() {
            rules.push_str(format!("{},", rule.to_domain_object_code(named_patterns)).as_str());
        }
        format!("&[{}]", rules)
    }
}

#[derive(Deserialize)]
struct RuleDTO {
    language: MaybeMany<String>,
    #[serde(flatten)]
    pattern: Option<PatternDTO>,
}

impl RuleDTO {
    fn to_domain_object_code(&self, named_patterns: &NamedPatterns) -> String {
        let languages = match &self.language {
            MaybeMany::Many(values) => values.clone(),
            MaybeMany::One(value) => vec![value.clone()],
        };

        let pattern_code = match &self.pattern {
            Some(pattern) => format!("Some({})", pattern.to_domain_object_code(named_patterns)),
            None => String::from("None"),
        };

        format!(
            "Rule {{ languages: &[\"{}\"], pattern: {}}}",
            languages.join("\",\""),
            pattern_code
        )
    }
}

#[derive(Clone, Deserialize)]
enum PatternDTO {
    #[serde(rename = "and")]
    And(Vec<PatternDTO>),
    #[serde(rename = "named_pattern")]
    Named(String),
    #[serde(rename = "negative_pattern")]
    Negative(String),
    #[serde(rename = "pattern")]
    Positive(MaybeMany<String>),
}

impl PatternDTO {
    fn to_domain_object_code(&self, named_patterns: &NamedPatterns) -> String {
        match self {
            PatternDTO::Positive(MaybeMany::One(pattern)) => {
                // Panic on invalid regex now so we can unwrap in lib
                if let Err(e) = PCRERegex::new(pattern) {
                    panic!("Invalid regex pattern: {}\n{}", pattern, e);
                }
                format!("Pattern::Positive({:?})", pattern)
            }
            PatternDTO::Negative(pattern) => {
                // Panic on invalid regex now so we can unwrap in lib
                if let Err(e) = PCRERegex::new(pattern) {
                    panic!("Invalid regex pattern: {}\n{}", pattern, e);
                }
                format!("Pattern::Negative({:?})", pattern)
            }
            PatternDTO::Positive(MaybeMany::Many(patterns)) => {
                let mut code = String::from("Pattern::Or(&[");
                for pattern in patterns.iter() {
                    let p = PatternDTO::Positive(MaybeMany::One(pattern.clone()));
                    code.push_str(format!("{},", p.to_domain_object_code(named_patterns)).as_str());
                }
                code.push_str("])");
                code
            }
            PatternDTO::And(patterns) => {
                let mut code = String::from("Pattern::And(&[");
                for pattern in patterns.iter() {
                    code.push_str(
                        format!("{},", pattern.to_domain_object_code(named_patterns)).as_str(),
                    );
                }
                code.push_str("])");
                code
            }
            PatternDTO::Named(pattern_name) => {
                if let Some(pattern) = named_patterns.get(pattern_name) {
                    // Assume that all named patterns are positive
                    let pattern = PatternDTO::Positive(pattern.clone());
                    return pattern.to_domain_object_code(named_patterns);
                } else {
                    panic!(
                        "Named pattern: {} not found in named pattern map",
                        pattern_name
                    );
                };
            }
        }
    }
}

#[derive(Clone, Deserialize)]
#[serde(untagged)]
enum MaybeMany<T> {
    Many(Vec<T>),
    One(T),
}

const DISAMBIGUATION_HEURISTICS_FILE: &str = "src/codegen/disambiguation-heuristics-map.rs";
const EXTENSION_MAP_FILE: &str = "src/codegen/extension-language-map.rs";
const FILENAME_MAP_FILE: &str = "src/codegen/filename-language-map.rs";
const INTERPRETER_MAP_FILE: &str = "src/codegen/interpreter-language-map.rs";
const LANGUAGE_INFO_FILE: &str = "src/codegen/language-info-map.rs";
const LANGUAGE_LIST_FILE: &str = "src/codegen/languages.rs";
const TOKEN_LOG_PROBABILITY_FILE: &str = "src/codegen/token-log-probabilities.rs";

const HEURISTICS_SOURCE_FILE: &str = "heuristics.yml";
const LANGUAGE_SOURCE_FILE: &str = "languages.yml";

const MAX_TOKEN_BYTES: usize = 32;

fn main() {
    let languages: LanguageMap =
        serde_yaml::from_reader(File::open(LANGUAGE_SOURCE_FILE).unwrap()).unwrap();

    write_language_list(&languages);
    write_language_info(&languages);
    create_filename_map(&languages);
    create_interpreter_map(&languages);
    create_extension_map(&languages);

    let heuristics: Heuristics =
        serde_yaml::from_str(&fs::read_to_string(HEURISTICS_SOURCE_FILE).unwrap()[..]).unwrap();
    create_disambiguation_heuristics_map(heuristics);

    train_classifier();
}

fn write_language_list(languages: &LanguageMap) {
    let mut file = BufWriter::new(File::create(LANGUAGE_LIST_FILE).unwrap());

    let languages: Vec<String> = languages.keys().map(|language| language.clone()).collect();

    writeln!(
        &mut file,
        "static LANGUAGES: &[&'static str] = &[\"{}\"];",
        languages.join("\",\"")
    )
    .unwrap();
}

fn write_language_info(languages: &LanguageMap) {
    let mut file = BufWriter::new(File::create(LANGUAGE_INFO_FILE).unwrap());

    let mut language_info_map = PhfMap::new();
    for (language_name, language) in languages.iter() {
        language_info_map.entry(
            &language_name[..],
            &language.to_domain_object_code(&language_name[..])[..],
        );
    }

    writeln!(
        &mut file,
        "static LANGUAGE_INFO: phf::Map<&'static str, Language> =\n{};\n",
        language_info_map.build()
    )
    .unwrap();
}

fn create_filename_map(languages: &LanguageMap) {
    let mut file = BufWriter::new(File::create(FILENAME_MAP_FILE).unwrap());

    let mut filename_to_language_map = PhfMap::new();
    for (language_name, language) in languages.iter() {
        if let Some(filenames) = &language.filenames {
            for filename in filenames.iter() {
                filename_to_language_map
                    .entry(&filename[..], &format!("\"{}\"", language_name)[..]);
            }
        }
    }

    writeln!(
        &mut file,
        "static FILENAMES: phf::Map<&'static str, &'static str> =\n{};\n",
        filename_to_language_map.build()
    )
    .unwrap();
}

fn create_interpreter_map(languages: &LanguageMap) {
    let mut file = BufWriter::new(File::create(INTERPRETER_MAP_FILE).unwrap());

    let mut temp_map: HashMap<&String, Vec<&String>> = HashMap::new();
    for (language_name, language) in languages.iter() {
        if let Some(interpreters) = &language.interpreters {
            for interpreter in interpreters.iter() {
                match temp_map.get_mut(interpreter) {
                    Some(entry) => {
                        entry.push(language_name);
                    }
                    None => {
                        temp_map.insert(interpreter, vec![language_name]);
                    }
                }
            }
        }
    }

    let mut interpreter_to_language_map = PhfMap::new();
    for (interpreter, languages) in temp_map.iter() {
        interpreter_to_language_map.entry(&interpreter[..], &format!("&{:?}", languages)[..]);
    }

    writeln!(
        &mut file,
        "static INTERPRETERS: phf::Map<&'static str, &[&str]> =\n{};\n",
        interpreter_to_language_map.build()
    )
    .unwrap();
}

fn create_extension_map(languages: &LanguageMap) {
    let mut file = BufWriter::new(File::create(EXTENSION_MAP_FILE).unwrap());

    let mut temp_map: HashMap<String, Vec<&String>> = HashMap::new();
    for (language_name, language) in languages.iter() {
        if let Some(extensions) = &language.extensions {
            for extension in extensions.iter() {
                let extension = extension.clone().to_ascii_lowercase();
                match temp_map.get_mut(&extension) {
                    Some(entry) => {
                        entry.push(language_name);
                    }
                    None => {
                        temp_map.insert(extension.clone(), vec![language_name]);
                    }
                }
            }
        }
    }

    let mut extension_to_language_map = PhfMap::new();
    for (extension, languages) in temp_map.iter() {
        extension_to_language_map.entry(&extension[..], &format!("&{:?}", languages)[..]);
    }

    writeln!(
        &mut file,
        "static EXTENSIONS: phf::Map<&'static str, &[&str]> =\n{};\n",
        extension_to_language_map.build()
    )
    .unwrap();
}

fn create_disambiguation_heuristics_map(heuristics: Heuristics) {
    let mut file = BufWriter::new(File::create(DISAMBIGUATION_HEURISTICS_FILE).unwrap());

    let mut temp_map: HashMap<String, String> = HashMap::new();
    for mut dis in heuristics.disambiguations.into_iter() {
        for ext in dis.extensions.iter() {
            // Adding a rule to default to C for .h if the Objective C and C++ patterns don't match
            // The classifer was unreliable for distinguishing between C and C++ for .h
            if ext == ".h" {
                dis.rules.push(RuleDTO {
                    language: MaybeMany::One(String::from("C")),
                    pattern: None,
                });
            }
            let extension = ext.clone().to_ascii_lowercase();
            let key = extension;
            let value = dis.to_domain_object_code(&heuristics.named_patterns);
            temp_map.insert(key, value);
        }
    }

    let mut disambiguation_heuristic_map = PhfMap::new();
    for (key, value) in temp_map.iter() {
        disambiguation_heuristic_map.entry(&key[..], &value[..]);
    }

    writeln!(
        &mut file,
        "static DISAMBIGUATIONS: phf::Map<&'static str, &'static [Rule]> =\n{};\n",
        disambiguation_heuristic_map.build()
    )
    .unwrap();
}

fn train_classifier() {
    let mut temp_token_count: HashMap<String, HashMap<String, i32>> = HashMap::new();
    let mut temp_total_tokens_count = HashMap::new();

    fs::read_dir("samples")
        .unwrap()
        .map(|entry| entry.unwrap())
        .filter(|entry| entry.path().is_dir())
        .map(|language_dir| {
            let path = language_dir.path();
            let language = path.file_name().unwrap();
            let language = language.to_string_lossy().into_owned();
            let language = match &language[..] {
                "Fstar" => String::from("F*"),
                _ => language,
            };

            let file_paths = fs::read_dir(language_dir.path())
                .unwrap()
                .map(|entry| entry.unwrap().path())
                .filter(|path| path.is_file());

            let language_iter = iter::repeat(language);
            file_paths.zip(language_iter)
        })
        .flatten()
        .for_each(|(entry, language)| {
            let content = fs::read(entry).unwrap();

            // When tokenizing an invalid utf8 string, just set it to ""
            // Add better error handling here in the future but unure of the best
            // way to handle it now
            let tokens =
                polyglot_tokenizer::get_key_tokens(std::str::from_utf8(&content[..]).unwrap_or(""));

            for token in tokens {
                if token.len() <= MAX_TOKEN_BYTES {
                    let total_tokens = temp_total_tokens_count.entry(language.clone()).or_insert(0);
                    *total_tokens += 1;

                    let tokens_count = temp_token_count
                        .entry(language.clone())
                        .or_insert(HashMap::new());

                    let count = tokens_count.entry(String::from(token)).or_insert(0);
                    *count += 1;
                }
            }
        });

    // Write token log probabilities
    let mut file = BufWriter::new(File::create(TOKEN_LOG_PROBABILITY_FILE).unwrap());
    let mut language_token_log_probabilities = PhfMap::new();
    for (language, token_count_map) in temp_token_count.iter() {
        let total_tokens = *temp_total_tokens_count.get(language).unwrap() as f64;
        let mut token_log_probabilities = PhfMap::new();
        for (token, token_count) in token_count_map.iter() {
            let probability = (*token_count as f64) / (total_tokens);
            let log_probability = probability.ln();
            token_log_probabilities.entry(&token[..], &format!("{}f64", log_probability)[..]);
        }
        let codegen_log_prob_map = format!("{}", token_log_probabilities.build());
        language_token_log_probabilities.entry(&language[..], &codegen_log_prob_map[..]);
    }

    writeln!(
        &mut file,
        "static TOKEN_LOG_PROBABILITIES: phf::Map<&'static str, phf::Map<&'static str, f64>> =\n{};\n",
        language_token_log_probabilities.build()
    )
    .unwrap();
}
