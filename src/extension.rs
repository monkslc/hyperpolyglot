// Include the map from interpreters to languages at compile time
// static EXTENSIONS: phf::Map<&'static str, &[&str]> = ...;
include!(concat!(env!("OUT_DIR"), "/extension-language-map.rs"));

pub fn get_language_by_extension(filename: &str) -> Vec<&'static str> {
    let extension = get_extension(filename);
    let extension = extension.as_str();
    let languages = EXTENSIONS
        .get(extension)
        .map(|languages| languages.to_vec());

    match languages {
        Some(languages) => languages,
        None => vec![],
    }
}

pub fn get_extension(filename: &str) -> String {
    let filename = if filename.starts_with(".") {
        &filename[1..]
    } else {
        filename
    };

    let extension_parts: Vec<&str> = filename.split(".").skip(1).collect();
    extension_parts.join(".")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_language_by_extension() {
        assert_eq!(get_language_by_extension("index.djs"), vec!["Dogescript"]);
        assert_eq!(get_language_by_extension("example.cmake.in"), vec!["CMake"]);

        let mut header_file_langs = get_language_by_extension("level.h");
        header_file_langs.sort();
        assert_eq!(header_file_langs, vec!["C", "C++", "Objective-C"]);

        let empty_vec: Vec<&'static str> = vec![];
        assert_eq!(get_language_by_extension("hello.kasdjf"), empty_vec);

        assert_eq!(get_language_by_extension(".c"), empty_vec);
        assert_eq!(get_language_by_extension(""), empty_vec);
        assert_eq!(get_language_by_extension("noextension"), empty_vec);
    }
}
