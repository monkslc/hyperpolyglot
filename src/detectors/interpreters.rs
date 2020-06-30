use lazy_static::lazy_static;
use regex::Regex;

// Include the map from interpreters to languages at compile time
// static INTERPRETERS: phf::Map<&'static str, &[&str]> = ...;
include!("../codegen/interpreter-language-map.rs");

pub fn get_languages_from_shebang<R: std::io::BufRead>(
    reader: R,
) -> Result<Vec<&'static str>, std::io::Error> {
    let mut lines = reader.lines();
    let shebang_line = match lines.next() {
        Some(line) => line,
        None => return Ok(vec![]),
    }?;
    let mut extra_content = String::new();

    if !shebang_line.starts_with("#!") {
        return Ok(vec![]);
    }

    let languages = shebang_line
        .split('/')
        .last()
        .and_then(|interpreter_line| {
            let mut splits = interpreter_line.split_whitespace();
            match splits.next() {
                // #!/usr/bin/env python
                Some("env") => splits.next(),
                // #!/usr/bin/sh [exec scala "$0" "$@"]
                Some("sh") => {
                    let lines: Vec<String> = lines.take(4).filter_map(|line| line.ok()).collect();
                    extra_content = lines.join("\n");
                    lazy_static! {
                        static ref SHEBANG_HACK_RE: Regex =
                            Regex::new(r#"exec (\w+).+\$0.+\$@"#).unwrap();
                    }
                    let interpreter = SHEBANG_HACK_RE
                        .captures(&extra_content[..])
                        .and_then(|captures| captures.get(1))
                        .map(|interpreter| interpreter.as_str())
                        .unwrap_or("sh");
                    Some(interpreter)
                }
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
    fn test_shebang_get_languages() {
        assert_eq!(
            get_languages_from_shebang(Cursor::new("#!/usr/bin/python")).unwrap(),
            vec!["Python"]
        );
    }
    #[test]
    fn test_shebang_get_languages_env() {
        assert_eq!(
            get_languages_from_shebang(Cursor::new("#!/usr/bin/env node")).unwrap(),
            vec!["JavaScript"]
        );
    }

    #[test]
    fn test_shebang_get_languages_multiple() {
        let mut parrot_langs =
            get_languages_from_shebang(Cursor::new("#!/usr/bin/parrot")).unwrap();
        parrot_langs.sort();
        assert_eq!(
            parrot_langs,
            vec!["Parrot Assembly", "Parrot Internal Representation"]
        );
    }

    #[test]
    fn test_shebang_get_languages_with_minor_version() {
        assert_eq!(
            get_languages_from_shebang(Cursor::new("#!/usr/bin/python2.6")).unwrap(),
            vec!["Python"]
        );
    }

    #[test]
    fn test_shebang_empty_cases() {
        let empty_vec: Vec<&'static str> = Vec::new();
        assert_eq!(
            get_languages_from_shebang(Cursor::new("#!/usr/bin/env")).unwrap(),
            empty_vec
        );
        assert_eq!(
            get_languages_from_shebang(Cursor::new("#!")).unwrap(),
            empty_vec
        );
        assert_eq!(
            get_languages_from_shebang(Cursor::new("")).unwrap(),
            empty_vec
        );
        assert_eq!(
            get_languages_from_shebang(Cursor::new("aslkdfjas;ldk")).unwrap(),
            empty_vec
        );
        assert_eq!(
            get_languages_from_shebang(Cursor::new(" #!/usr/bin/python")).unwrap(),
            empty_vec
        );
        assert_eq!(
            get_languages_from_shebang(Cursor::new(" #!/usr/bin/ ")).unwrap(),
            empty_vec
        );
        assert_eq!(
            get_languages_from_shebang(Cursor::new(" #!/usr/bin")).unwrap(),
            empty_vec
        );
        assert_eq!(
            get_languages_from_shebang(Cursor::new(" #!/usr/bin")).unwrap(),
            empty_vec
        );
        assert_eq!(
            get_languages_from_shebang(Cursor::new("")).unwrap(),
            empty_vec
        );
    }

    #[test]
    fn test_shebang_hack() {
        let content = Cursor::new(
            r#"#!/bin/sh
               exec scala "$0" "$@"
               !#
            "#,
        );

        assert_eq!(get_languages_from_shebang(content).unwrap(), vec!["Scala"]);
    }
}
