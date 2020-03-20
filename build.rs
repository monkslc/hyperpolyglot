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

fn main() {
    let languages: HashMap<String, Language> =
        serde_yaml::from_str(&fs::read_to_string("languages.yml").unwrap()[..]).unwrap();

    create_filename_map(&languages);
    create_interpreter_map(&languages);
    create_extension_map(&languages);
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
        interpreter_to_language_map.entry(
            &interpreter[..],
            // split langauges with a | character
            format!("\"{}\"", languages.join("|")).as_str(),
        );
    }

    writeln!(
        &mut file,
        "static INTERPRETERS: phf::Map<&'static str, &'static str> = \n{};\n",
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
        extension_to_language_map.entry(
            &extension[..],
            // split langauges with a | character
            format!("\"{}\"", languages.join("|")).as_str(),
        );
    }

    writeln!(
        &mut file,
        "static EXTENSIONS: phf::Map<&'static str, &'static str> = \n{};\n",
        extension_to_language_map.build()
    )
    .unwrap();
}
