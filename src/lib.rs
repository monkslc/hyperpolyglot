use lazy_static::lazy_static;
use regex::Regex;
use std::io::{self, BufRead, BufReader, Read};

// Include the map from filenames to languages at compile time
// static FILENAMES: phf::Map<&'static str, &'static str> = ...;
include!(concat!(env!("OUT_DIR"), "/filename-language-map.rs"));

// Include the map from interpreters to languages at compile time
// static INTERPRETERS: phf::Map<&'static str, &[&str]> = ...;
include!(concat!(env!("OUT_DIR"), "/interpreter-language-map.rs"));

// Include the map from interpreters to languages at compile time
// static EXTENSIONS: phf::Map<&'static str, &[&str]> = ...;
include!(concat!(env!("OUT_DIR"), "/extension-language-map.rs"));

pub fn get_language_by_filename(filename: &str) -> Option<&&'static str> {
    FILENAMES.get(filename)
}

pub fn get_language_by_shebang<R: Read>(reader: R) -> Result<Vec<&'static str>, io::Error> {
    let mut reader = BufReader::new(reader);
    let mut line = String::new();
    reader.read_line(&mut line)?;
    if !line.starts_with("#!") {
        return Ok(vec![]);
    }

    let languages = line
        .split("/")
        .last()
        .and_then(|interpreter_line| {
            let mut splits = interpreter_line.split_whitespace();
            match splits.next() {
                // #!/usr/bin/env python
                Some("env") => splits.next(),
                // #!/usr/bin/python
                Some(interpreter) => Some(interpreter),
                // #!
                None => None,
            }
        })
        .and_then(|interpreter| {
            // #!/usr/bin/python2.6.3 -> #!/usr/bin/python2
            lazy_static! {
                static ref RE: Regex = Regex::new(r#"[0-9]\.[0-9]"#).unwrap();
            }
            let interpreter = RE.split(interpreter).next().unwrap();

            INTERPRETERS.get(interpreter)
        });

    match languages {
        Some(languages) => Ok(languages.to_vec()),
        None => Ok(vec![]),
    }
}

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

fn get_extension(filename: &str) -> String {
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
    use std::io::Cursor;

    #[test]
    fn test_get_language_by_filename() {
        assert_eq!(get_language_by_filename("APKBUILD"), Some(&"Alpine Abuild"));
        assert_eq!(
            get_language_by_filename(".eslintrc.json"),
            Some(&"JSON with Comments")
        );
    }

    #[test]
    fn test_get_language_by_shebang() {
        assert_eq!(
            get_language_by_shebang(Cursor::new("#!/usr/bin/python")).unwrap(),
            vec!["Python"]
        );

        assert_eq!(
            get_language_by_shebang(Cursor::new("#!/usr/bin/env node")).unwrap(),
            vec!["JavaScript"]
        );

        let mut parrot_langs = get_language_by_shebang(Cursor::new("#!/usr/bin/parrot")).unwrap();
        parrot_langs.sort();
        assert_eq!(
            parrot_langs,
            vec!["Parrot Assembly", "Parrot Internal Representation"]
        );

        assert_eq!(
            get_language_by_shebang(Cursor::new("#!/usr/bin/python2.6")).unwrap(),
            vec!["Python"]
        );

        let empty_vec: Vec<&'static str> = Vec::new();
        assert_eq!(
            get_language_by_shebang(Cursor::new("#!/usr/bin/env")).unwrap(),
            empty_vec
        );

        assert_eq!(
            get_language_by_shebang(Cursor::new("#!")).unwrap(),
            empty_vec
        );

        assert_eq!(get_language_by_shebang(Cursor::new("")).unwrap(), empty_vec);
        assert_eq!(
            get_language_by_shebang(Cursor::new("aslkdfjas;ldk")).unwrap(),
            empty_vec
        );

        assert_eq!(
            get_language_by_shebang(Cursor::new(" #!/usr/bin/python")).unwrap(),
            empty_vec
        );

        assert_eq!(
            get_language_by_shebang(Cursor::new(" #!/usr/bin/ ")).unwrap(),
            empty_vec
        );

        assert_eq!(
            get_language_by_shebang(Cursor::new(" #!/usr/bin")).unwrap(),
            empty_vec
        );
    }

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
