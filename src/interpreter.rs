use lazy_static::lazy_static;
use regex::Regex;
use std::io::{self, BufRead, BufReader, Read};

// Include the map from interpreters to languages at compile time
// static INTERPRETERS: phf::Map<&'static str, &[&str]> = ...;
include!(concat!(env!("OUT_DIR"), "/interpreter-language-map.rs"));

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

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
}
