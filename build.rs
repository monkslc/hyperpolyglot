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
}

fn main() {
    let path = Path::new(&env::var("OUT_DIR").unwrap()).join("filename-language-map.rs");
    let mut file = BufWriter::new(File::create(&path).unwrap());
    let languages: HashMap<String, Language> =
        serde_yaml::from_str(&fs::read_to_string("languages.yml").unwrap()[..]).unwrap();

    let mut filename_to_language_map = phf_codegen::Map::new();

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
