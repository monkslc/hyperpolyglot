use lazy_static::lazy_static;
use regex::Regex;

// Include the map from interpreters to languages at compile time
// static INTERPRETERS: phf::Map<&'static str, &[&str]> = ...;
include!("codegen/interpreter-language-map.rs");

pub fn get_language_by_shebang(shebang_line: &str) -> Vec<&'static str> {
    if !shebang_line.starts_with("#!") {
        return vec![];
    }

    let languages = shebang_line
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
        Some(languages) => languages.to_vec(),
        None => vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_language_by_shebang() {
        assert_eq!(get_language_by_shebang("#!/usr/bin/python"), vec!["Python"]);

        assert_eq!(
            get_language_by_shebang("#!/usr/bin/env node"),
            vec!["JavaScript"]
        );

        let mut parrot_langs = get_language_by_shebang("#!/usr/bin/parrot");
        parrot_langs.sort();
        assert_eq!(
            parrot_langs,
            vec!["Parrot Assembly", "Parrot Internal Representation"]
        );

        assert_eq!(
            get_language_by_shebang("#!/usr/bin/python2.6"),
            vec!["Python"]
        );

        let empty_vec: Vec<&'static str> = Vec::new();
        assert_eq!(get_language_by_shebang("#!/usr/bin/env"), empty_vec);
        assert_eq!(get_language_by_shebang("#!"), empty_vec);
        assert_eq!(get_language_by_shebang(""), empty_vec);
        assert_eq!(get_language_by_shebang("aslkdfjas;ldk"), empty_vec);
        assert_eq!(get_language_by_shebang(" #!/usr/bin/python"), empty_vec);
        assert_eq!(get_language_by_shebang(" #!/usr/bin/ "), empty_vec);
        assert_eq!(get_language_by_shebang(" #!/usr/bin"), empty_vec);
        assert_eq!(get_language_by_shebang(" #!/usr/bin"), empty_vec);
        assert_eq!(get_language_by_shebang(""), empty_vec);
    }
}
