// Include the map from extensions to languages at compile time
// static EXTENSIONS: phf::Map<&'static str, &[&str]> = ...;
include!("codegen/extension-language-map.rs");

pub fn get_language(extension: &str) -> Vec<&'static str> {
    let languages = EXTENSIONS
        .get(extension)
        .map(|languages| languages.to_vec());

    match languages {
        Some(languages) => languages,
        None => vec![],
    }
}

pub fn get(filename: &str) -> Option<&'static str> {
    let filename = if filename.starts_with(".") {
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
    fn test_get_language() {
        assert_eq!(get_language(".djs"), vec!["Dogescript"]);
        assert_eq!(get_language(".cmake.in"), vec!["CMake"]);

        let mut header_file_langs = get_language(".h");
        header_file_langs.sort();
        assert_eq!(header_file_langs, vec!["C", "C++", "Objective-C"]);

        let empty_vec: Vec<&'static str> = vec![];
        assert_eq!(get_language(""), empty_vec);
    }

    #[test]
    fn test_get() {
        assert_eq!(get("index.djs"), Some(".djs"));
        assert_eq!(get("example.cmake.in"), Some(".cmake.in"));
        assert_eq!(get("nonsense.notrealextension.c"), Some(".c"));
        assert_eq!(get("uppercase.C"), Some(".c"));
        assert_eq!(get(".eslintrc.json"), Some(".json"));
        assert_eq!(get(".cs"), None);
        assert_eq!(get("noextension"), None);
    }
}
