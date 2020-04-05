// Include the map from interpreters to languages at compile time
// static EXTENSIONS: phf::Map<&'static str, &[&str]> = ...;
include!("codegen/extension-language-map.rs");

pub fn get_language_by_extension(extension: &str) -> Vec<&'static str> {
    let languages = EXTENSIONS
        .get(extension)
        .map(|languages| languages.to_vec());

    match languages {
        Some(languages) => languages,
        None => vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_language_by_extension() {
        assert_eq!(get_language_by_extension("djs"), vec!["Dogescript"]);
        assert_eq!(get_language_by_extension("cmake.in"), vec!["CMake"]);

        let mut header_file_langs = get_language_by_extension("h");
        header_file_langs.sort();
        assert_eq!(header_file_langs, vec!["C", "C++", "Objective-C"]);

        let empty_vec: Vec<&'static str> = vec![];
        assert_eq!(get_language_by_extension(""), empty_vec);
    }
}
