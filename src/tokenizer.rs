use logos::Logos;

#[derive(Logos, Debug, PartialEq)]
enum Token<'a> {
    #[regex("[A-Za-z0-9_]+", |lex| lex.slice())]
    Text(&'a str),

    #[regex("[^A-Za-z0-9_\\s]", |lex| lex.slice())]
    Symbol(&'a str),

    #[error]
    Error,
}

pub fn tokenize(content: &str) -> impl Iterator<Item = &str> {
    let iter = Token::lexer(content);
    iter.filter_map(|t| match t {
        Token::Text(t) | Token::Symbol(t) => Some(t),
        _ => None,
    })
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

        let tokens = tokenize(sample);
        let expected = vec![
            "fn", "main", "(", ")", "{", "let", "x_x", "=", "5", ";", "println", "!", "(", "\"",
            "{", "}", "\"", ",", "x", ")", ";", "}",
        ];
        for (i, token) in tokens.enumerate() {
            assert_eq!(token, expected[i]);
        }
    }

    #[test]
    fn test_empty_tokenizer_string() {
        let tokens: Vec<&str> = tokenize("").collect();
        assert_eq!(tokens.len(), 0);
    }
}
