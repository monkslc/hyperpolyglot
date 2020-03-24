use pcre2::bytes::Regex as PCRERegex;
use phf_codegen::Map as PhfMap;
use serde::Deserialize;
use std::{
    collections::HashMap,
    env,
    fs::{self, File},
    io::{BufWriter, Write},
    path::Path,
};

#[derive(Deserialize)]
struct Language {
    filenames: Option<Vec<String>>,
    interpreters: Option<Vec<String>>,
    extensions: Option<Vec<String>>,
}

type NamedPatterns = HashMap<String, MaybeMany<String>>;
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

fn main() {
    let languages: HashMap<String, Language> =
        serde_yaml::from_str(&fs::read_to_string("languages.yml").unwrap()[..]).unwrap();

    create_filename_map(&languages);
    create_interpreter_map(&languages);
    create_extension_map(&languages);

    let heuristics: Heuristics =
        serde_yaml::from_str(&fs::read_to_string("heuristics.yml").unwrap()[..]).unwrap();
    create_disambiguation_heuristics_map(&heuristics);
}

fn create_filename_map(languages: &HashMap<String, Language>) {
    let path = Path::new(&env::var("OUT_DIR").unwrap()).join("filename-language-map.rs");
    let mut file = BufWriter::new(File::create(&path).unwrap());

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
        "static FILENAMES: phf::Map<&'static str, &'static str> = \n{};\n",
        filename_to_language_map.build()
    )
    .unwrap();
}

fn create_interpreter_map(languages: &HashMap<String, Language>) {
    let path = Path::new(&env::var("OUT_DIR").unwrap()).join("interpreter-language-map.rs");
    let mut file = BufWriter::new(File::create(&path).unwrap());

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
        "static INTERPRETERS: phf::Map<&'static str, &[&str]> = \n{};\n",
        interpreter_to_language_map.build()
    )
    .unwrap();
}

fn create_extension_map(languages: &HashMap<String, Language>) {
    let path = Path::new(&env::var("OUT_DIR").unwrap()).join("extension-language-map.rs");
    let mut file = BufWriter::new(File::create(&path).unwrap());

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
        "static EXTENSIONS: phf::Map<&'static str, &[&str]> = \n{};\n",
        extension_to_language_map.build()
    )
    .unwrap();
}

fn create_disambiguation_heuristics_map(heuristics: &Heuristics) {
    let path = Path::new(&env::var("OUT_DIR").unwrap()).join("disambiguation-heuristics-map.rs");
    let mut file = BufWriter::new(File::create(&path).unwrap());

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
        "static DISAMBIGUATIONS: phf::Map<&'static str, &'static [Rule]> = \n{};\n",
        disambiguation_heuristic_map.build()
    )
    .unwrap();
}
