use pcre2::bytes::Regex as PCRERegex;
use phf_codegen::Map as PhfMap;
use serde::Deserialize;
use std::{
    collections::HashMap,
    fs::{self, File},
    io::{self, BufWriter, Write},
    iter, time,
};

type LanguageMap = HashMap<String, Language>;
type NamedPatterns = HashMap<String, MaybeMany<String>>;

#[derive(Deserialize)]
struct Language {
    filenames: Option<Vec<String>>,
    interpreters: Option<Vec<String>>,
    extensions: Option<Vec<String>>,
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
        // If we have more than one language, take the first
        // The only case this happens is the [Linux Kernel Module, AMPL] for .mod extension
        // And I'm not sure what the right behavior is in that case
        let language = match &self.language {
            MaybeMany::Many(values) => &values[0],
            MaybeMany::One(value) => value,
        };

        let pattern_code = match &self.pattern {
            Some(pattern) => format!("Some({})", pattern.to_domain_object_code(named_patterns)),
            None => String::from("None"),
        };

        format!(
            "Rule {{ language: \"{}\", pattern: {}}}",
            language, pattern_code
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
const LANGUAGE_LIST_FILE: &str = "src/codegen/languages.rs";
const LANGUAGE_TOKEN_COUNT_FILE: &str = "src/codegen/language-token-count.rs";
const TOTAL_TOKEN_COUNT_FILE: &str = "src/codegen/total-token-count.rs";

const HEURISTICS_SOURCE_FILE: &str = "heuristics.yml";
const LANGUAGE_SOURCE_FILE: &str = "languages.yml";
const SAMPLES_DIR: &str = "samples";

fn main() {
    let languages_last_updated = get_last_updated_time(&LANGUAGE_SOURCE_FILE).unwrap();
    let languages: LanguageMap =
        serde_yaml::from_str(&fs::read_to_string(&LANGUAGE_SOURCE_FILE).unwrap()[..]).unwrap();

    let language_dependents: Vec<(&dyn Fn(&LanguageMap) -> (), &str)> = vec![
        (&write_languages, LANGUAGE_LIST_FILE),
        (&create_filename_map, FILENAME_MAP_FILE),
        (&create_interpreter_map, INTERPRETER_MAP_FILE),
        (&create_extension_map, EXTENSION_MAP_FILE),
    ];
    language_dependents.iter().for_each(|(func, codegen_file)| {
        if should_update_codegen(languages_last_updated, codegen_file) {
            func(&languages);
        }
    });

    let heuristics_last_updated = get_last_updated_time(HEURISTICS_SOURCE_FILE).unwrap();
    let heuristics: Heuristics =
        serde_yaml::from_str(&fs::read_to_string(HEURISTICS_SOURCE_FILE).unwrap()[..]).unwrap();
    if should_update_codegen(heuristics_last_updated, DISAMBIGUATION_HEURISTICS_FILE) {
        create_disambiguation_heuristics_map(&heuristics);
    };

    let samples_last_updated = get_last_updated_time(SAMPLES_DIR).unwrap();
    if should_update_codegen(samples_last_updated, LANGUAGE_TOKEN_COUNT_FILE)
        || should_update_codegen(samples_last_updated, TOTAL_TOKEN_COUNT_FILE)
    {
        train_classifier();
    }
}

fn write_languages(languages: &LanguageMap) {
    let mut file = BufWriter::new(File::create(LANGUAGE_LIST_FILE).unwrap());

    let languages: Vec<String> = languages.keys().map(|language| language.clone()).collect();

    writeln!(
        &mut file,
        "static LANGUAGES: &[&'static str] = &[\"{}\"];",
        languages.join("\",\"")
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
                    .entry(&filename[..], format!("\"{}\"", language_name).as_str());
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

    let mut temp_map: HashMap<String, Vec<String>> = HashMap::new();
    for (language_name, language) in languages.iter() {
        if let Some(interpreters) = &language.interpreters {
            for interpreter in interpreters.iter() {
                match temp_map.get_mut(interpreter) {
                    Some(entry) => {
                        entry.push(language_name.clone());
                    }
                    None => {
                        temp_map.insert(interpreter.clone(), vec![language_name.clone()]);
                    }
                }
            }
        }
    }

    let mut interpreter_to_language_map = PhfMap::new();
    for (interpreter, languages) in temp_map.iter() {
        interpreter_to_language_map.entry(&interpreter[..], format!("&{:?}", languages).as_str());
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

    let mut temp_map: HashMap<String, Vec<String>> = HashMap::new();
    for (language_name, language) in languages.iter() {
        if let Some(extensions) = &language.extensions {
            for extension in extensions.iter() {
                let mut extension = extension.clone();
                // .js => js
                extension.remove(0);
                match temp_map.get_mut(&extension) {
                    Some(entry) => {
                        entry.push(language_name.clone());
                    }
                    None => {
                        temp_map.insert(extension.clone(), vec![language_name.clone()]);
                    }
                }
            }
        }
    }

    let mut extension_to_language_map = PhfMap::new();
    for (extension, languages) in temp_map.iter() {
        extension_to_language_map.entry(&extension[..], format!("&{:?}", languages).as_str());
    }

    writeln!(
        &mut file,
        "static EXTENSIONS: phf::Map<&'static str, &[&str]> =\n{};\n",
        extension_to_language_map.build()
    )
    .unwrap();
}

fn create_disambiguation_heuristics_map(heuristics: &Heuristics) {
    let mut file = BufWriter::new(File::create(DISAMBIGUATION_HEURISTICS_FILE).unwrap());

    let mut temp_map: HashMap<String, String> = HashMap::new();
    for dis in heuristics.disambiguations.iter() {
        for ext in dis.extensions.iter() {
            let mut extension = ext.clone();
            // .js => js
            extension.remove(0);
            let key = format!("{}", extension);
            let value = format!("{}", dis.to_domain_object_code(&heuristics.named_patterns));
            temp_map.insert(key, value);
        }
    }

    let mut disambiguation_heuristic_map = PhfMap::new();
    for (key, value) in temp_map.iter() {
        disambiguation_heuristic_map.entry(key.as_str(), value.as_str());
    }

    writeln!(
        &mut file,
        "static DISAMBIGUATIONS: phf::Map<&'static str, &'static [Rule]> =\n{};\n",
        disambiguation_heuristic_map.build()
    )
    .unwrap();
}

fn train_classifier() {
    let mut temp_language_token_count = HashMap::new();
    let mut temp_total_tokens_count = HashMap::new();

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
                .filter(|path| !path.is_dir());

            let language_iter = iter::repeat(language);
            file_paths.zip(language_iter)
        })
        .flatten()
        .for_each(|(entry, language)| {
            let content = fs::read(entry).unwrap();

            // When tokenizing an invalid utf8 string, just set it to ""
            // Add better error handling here in the future but unure of the best
            // way to handle it now
            let tokens = tokens::tokenize(std::str::from_utf8(&content[..]).unwrap_or("")).unwrap();

            for token in tokens {
                let key = format!("{}{}", &language, token);
                let count = temp_language_token_count.entry(key).or_insert(0);
                *count += 1;

                let total_tokens = temp_total_tokens_count.entry(language.clone()).or_insert(0);
                *total_tokens += 1;
            }
        });

    // Write language token count
    let mut file = BufWriter::new(File::create(LANGUAGE_TOKEN_COUNT_FILE).unwrap());
    let mut language_token_count = PhfMap::new();
    for (key, value) in temp_language_token_count.iter() {
        let value = format!("{}f64", value);
        language_token_count.entry(key.as_str(), value.as_str());
    }

    writeln!(
        &mut file,
        "static LANGUAGE_TOKEN_COUNT: phf::Map<&'static str, f64> =\n{};\n",
        language_token_count.build()
    )
    .unwrap();

    // Write total token counts
    let mut file = BufWriter::new(File::create(TOTAL_TOKEN_COUNT_FILE).unwrap());
    let mut total_token_count = PhfMap::new();
    for (key, value) in temp_total_tokens_count.iter() {
        let value = format!("{}f64", value);
        total_token_count.entry(key.as_str(), value.as_str());
    }

    writeln!(
        &mut file,
        "static TOTAL_TOKEN_COUNT: phf::Map<&'static str, f64> =\n{};\n",
        total_token_count.build()
    )
    .unwrap();
}

fn get_last_updated_time(path: &str) -> Result<time::SystemTime, io::Error> {
    File::open(path)?.metadata()?.modified()
}

fn should_update_codegen(source_last_updated: time::SystemTime, codegen_path: &str) -> bool {
    match get_last_updated_time(codegen_path) {
        Ok(time) => source_last_updated > time,
        Err(e) => match e.kind() {
            std::io::ErrorKind::NotFound => true,
            _ => panic!("{}", e),
        },
    }
}
