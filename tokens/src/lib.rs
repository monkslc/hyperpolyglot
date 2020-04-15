use lazy_static::lazy_static;

#[macro_use]
extern crate lalrpop_util;

lalrpop_mod!(pub tokenizer);

pub fn tokenize(content: &str) -> Result<Vec<&str>, String> {
    lazy_static! {
        static ref PARSER: tokenizer::TokensParser = tokenizer::TokensParser::new();
    }

    match PARSER.parse(&content[..]) {
        Ok(tokens) => Ok(tokens),
        Err(e) => Err(format!("{}", e)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenizer() {
        let sample = r#"
        fn main() {
            let x_x = 5;
            println!("{}", x);
        }
        "#;

        let tokens = tokenize(sample).unwrap();
        let expected = vec![
            "fn", "main", "(", ")", "{", "let", "x_x", "=", "5", ";", "println", "!", "(", "\"",
            "{", "}", "\"", ",", "x", ")", ";", "}",
        ];
        for (i, token) in tokens.iter().enumerate() {
            assert_eq!(*token, expected[i]);
        }
    }

    #[test]
    fn test_empty_tokenizer_string() {
        let tokens = tokenize("").unwrap();
        assert_eq!(tokens.len(), 0);
    }
}
