// Include the map from extensions to languages at compile time
// static EXTENSIONS: phf::Map<&'static str, &[&str]> = ...;
include!("../codegen/extension-language-map.rs");

pub fn get_languages_from_extension(extension: &str) -> Vec<&'static str> {
    let languages = EXTENSIONS
        .get(extension)
        .map(|languages| languages.to_vec());

    match languages {
        Some(languages) => languages,
        None => vec![],
    }
}

pub fn get_extension(filename: &str) -> Option<&'static str> {
    let filename = if filename.starts_with('.') {
        &filename[1..]
    } else {
        filename
    };

    let filename = filename.to_ascii_lowercase();
    for (pos, ch) in filename.char_indices() {
        if ch == '.' {
            if let Some(extension) = EXTENSIONS.get_key(&filename[pos..]) {
                return Some(extension);
            };
        };
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_languages_from_extension() {
        assert_eq!(get_languages_from_extension(".djs"), vec!["Dogescript"]);
        assert_eq!(get_languages_from_extension(".cmake.in"), vec!["CMake"]);

        let mut header_file_langs = get_languages_from_extension(".h");
        header_file_langs.sort();
        assert_eq!(header_file_langs, vec!["C", "C++", "Objective-C"]);

        let empty_vec: Vec<&'static str> = vec![];
        assert_eq!(get_languages_from_extension(""), empty_vec);
    }

    #[test]
    fn test_get_extension() {
        assert_eq!(get_extension("index.djs"), Some(".djs"));
        assert_eq!(get_extension("example.cmake.in"), Some(".cmake.in"));
        assert_eq!(get_extension("nonsense.notrealextension.c"), Some(".c"));
        assert_eq!(get_extension("uppercase.C"), Some(".c"));
        assert_eq!(get_extension(".eslintrc.json"), Some(".json"));
        assert_eq!(get_extension(".cs"), None);
        assert_eq!(get_extension("noextension"), None);
    }
}
