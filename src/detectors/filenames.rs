// Include the map from filenames to languages at compile time
// static FILENAMES: phf::Map<&'static str, &'static str> = ...;
include!("../codegen/filename-language-map.rs");

pub fn get_language_from_filename(filename: &str) -> Option<&'static str> {
    FILENAMES.get(filename).copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_language_from_filename() {
        assert_eq!(
            get_language_from_filename("APKBUILD"),
            Some("Alpine Abuild")
        );
        assert_eq!(
            get_language_from_filename(".eslintrc.json"),
            Some("JSON with Comments")
        );
    }
}
