// Include the map from filenames to languages at compile time
// static FILENAMES: phf::Map<&'static str, &'static str> = ...;
include!(concat!(env!("OUT_DIR"), "/filename-language-map.rs"));

pub fn get_language_by_filename(filename: &str) -> Option<&&'static str> {
    FILENAMES.get(filename)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_language_by_filename() {
        assert_eq!(get_language_by_filename("APKBUILD"), Some(&"Alpine Abuild"));
        assert_eq!(
            get_language_by_filename(".eslintrc.json"),
            Some(&"JSON with Comments")
        );
    }
}
