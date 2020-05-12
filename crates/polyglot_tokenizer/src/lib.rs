use logos::{Lexer, Logos};

struct Cycle<T: Copy> {
    items: Vec<T>,
    index: usize,
}

impl<T: Copy> Cycle<T> {
    fn new(starting_items: Vec<T>) -> Self {
        Cycle {
            items: starting_items,
            index: 0,
        }
    }

    fn push(&mut self, item: T) {
        self.items[self.index] = item;
        self.bump_index();
    }

    fn bump_index(&mut self) {
        self.index = (self.index + 1) % self.items.len();
    }

    fn get_items(&self) -> Vec<T> {
        let mut items = Vec::with_capacity(self.items.len());
        for i in 0..self.items.len() {
            items.push(self.items[(i + self.index) % self.items.len()])
        }
        items
    }
}

fn maybe_line_token<'a>(lex: &mut Lexer<'a, Token<'a>>, ending_char: char) -> &'a str {
    let chars = lex.remainder().chars();
    let mut prev_char = ' ';
    let mut char_bytes_seen = 0;
    let mut end_of_seq_byte_index = None;
    for ch in chars {
        char_bytes_seen += ch.len_utf8();
        if ch == ending_char && prev_char != '\\' {
            end_of_seq_byte_index = Some(char_bytes_seen);
            break;
        }

        if ch == '\n' {
            break;
        }

        prev_char = ch
    }

    let start = lex.span().start;
    let end = lex.span().end;
    let end_of_seq_byte_index = end_of_seq_byte_index.unwrap_or(0);
    lex.bump(end_of_seq_byte_index);
    &lex.source()[start..end + end_of_seq_byte_index]
}

fn maybe_block_token<'a>(lex: &mut Lexer<'a, Token<'a>>, end_seq: Vec<char>) -> &'a str {
    let chars = lex.remainder().chars();
    let mut prev_chars = Cycle::new(vec!['a'; end_seq.len()]);
    let mut char_bytes_seen = 0;
    let mut end_of_seq_byte_index = None;
    for ch in chars {
        char_bytes_seen += ch.len_utf8();
        prev_chars.push(ch);
        if prev_chars.get_items() == end_seq {
            end_of_seq_byte_index = Some(char_bytes_seen);
            break;
        }
    }

    let start = lex.span().start;
    let end = lex.span().end;
    let end_of_seq_byte_index = end_of_seq_byte_index.unwrap_or(0);
    lex.bump(end_of_seq_byte_index);
    &lex.source()[start..end + end_of_seq_byte_index]
}

fn parse_number<'a>(lex: &mut Lexer<'a, Token<'a>>) -> f64 {
    let sanitized_string: String = lex.slice().chars().filter(|ch| *ch != '_').collect();
    // overflow can cause this to fail, just set it to 0 in this case
    sanitized_string.parse().unwrap_or(0.0)
}

fn parse_binary_number<'a>(lex: &mut Lexer<'a, Token<'a>>) -> f64 {
    let sanitized_string: String = lex
        .slice()
        .replace("0b", "")
        .chars()
        .filter(|ch| *ch != '_')
        .collect();
    // overflow can cause this to fail, just set it to 0 in this case
    let int: isize = isize::from_str_radix(&sanitized_string[..], 2).unwrap_or(0);
    int as f64
}

fn parse_octal_number<'a>(lex: &mut Lexer<'a, Token<'a>>) -> f64 {
    let sanitized_string: String = lex
        .slice()
        .replace("0o", "")
        .chars()
        .filter(|ch| *ch != '_')
        .collect();
    // overflow can cause this to fail, just set it to 0 in this case
    let int: isize = isize::from_str_radix(&sanitized_string[..], 8).unwrap_or(0);
    int as f64
}

fn parse_hex_number<'a>(lex: &mut Lexer<'a, Token<'a>>) -> f64 {
    let sanitized_string: String = lex
        .slice()
        .replace("0x", "")
        .chars()
        .filter(|ch| *ch != '_')
        .collect();
    // overflow can cause this to fail, just set it to 0 in this case
    let int: isize = isize::from_str_radix(&sanitized_string[..], 16).unwrap_or(0);
    int as f64
}

#[derive(Logos, Debug, PartialEq)]
pub enum Token<'a> {
    #[regex("[A-Za-z0-9_]+", |lex| lex.slice())]
    Ident(&'a str),

    #[regex("[^A-Za-z0-9_\\s]", |lex| lex.slice())]
    Symbol(&'a str),

    #[regex("//.*", |lex| lex.slice())]
    #[regex("#.*", |lex| lex.slice())]
    #[regex("--.*", |lex| lex.slice())]
    #[regex("%.*", |lex| lex.slice())]
    LineComment(&'a str),

    #[token("/*", |lex| maybe_block_token(lex, vec!['*', '/']))]
    #[token("{-", |lex| maybe_block_token(lex, vec!['-', '}']))]
    #[token("(*", |lex| maybe_block_token(lex, vec!['*', ')']))]
    #[token("<!--", |lex| maybe_block_token(lex, vec!['-', '-', '>']))]
    BlockComment(&'a str),

    // Using char::from to avoid syntax highlighting issues from having an unterminated double quote
    #[regex("\"", |lex| maybe_line_token(lex, char::from(34)))]
    #[token("\"\"\"", |lex| maybe_block_token(lex, vec![char::from(34); 3]))]
    #[token("'''", |lex| maybe_block_token(lex, vec!['\''; 3]))]
    #[regex("\'", |lex| maybe_line_token(lex, '\''))]
    StringLiteral(&'a str),

    #[regex(r"[-+]?(?:[0-9][0-9_]*)?\.?([0-9][0-9_]*)", priority = 2, callback = parse_number)]
    #[regex(r"[-+]?0b[0-1_]+", priority = 2, callback = parse_binary_number)]
    #[regex(r"[-+]?0o[0-7_]+", priority = 2, callback = parse_octal_number)]
    #[regex(r"[-+]?0x[0-9A-Fa-f_]+", priority = 2, callback = parse_hex_number)]
    Number(f64),

    #[error]
    #[regex(r"[ \t\n\f]+", logos::skip)]
    Error,
}

/// Tokenize the content and return only the identifiers and symbols from the langauge
///
/// # Examples
/// ```
/// use polyglot_tokenizer;
/// let content = r#"let x = [5, "hello"];"#;
/// let tokens: Vec<&str> = polyglot_tokenizer::get_key_tokens(content).collect();
/// assert_eq!(tokens, vec!["let", "x", "=", "[", ",", "]", ";"]);
/// ```
pub fn get_key_tokens(content: &str) -> impl Iterator<Item = &str> {
    let iter = Token::lexer(content);
    iter.filter_map(|t| match t {
        Token::Ident(t) | Token::Symbol(t) => Some(t),
        _ => None,
    })
}

pub fn tokenize(content: &str) -> impl Iterator<Item = Token> {
    Token::lexer(content)
}

#[cfg(test)]
mod tests {
    use super::*;
    use Token::*;

    #[test]
    fn test_tokenizer() {
        let sample = r#"
        fn main() {
            let x_x = 5;
            println!("{}", x);
        }
        "#;

        let tokens: Vec<Token> = Token::lexer(sample).collect();
        let expected = vec![
            Ident("fn"),
            Ident("main"),
            Symbol("("),
            Symbol(")"),
            Symbol("{"),
            Ident("let"),
            Ident("x_x"),
            Symbol("="),
            Number(5.0),
            Symbol(";"),
            Ident("println"),
            Symbol("!"),
            Symbol("("),
            StringLiteral("\"{}\""),
            Symbol(","),
            Ident("x"),
            Symbol(")"),
            Symbol(";"),
            Symbol("}"),
        ];
        assert_eq!(tokens, expected)
    }

    #[test]
    fn test_string_literal() {
        let sample = r#"
            let x = ["Hello", 'Hello'];
            """ python """
            ''' python '''
        "#;

        let tokens: Vec<Token> = Token::lexer(sample).collect();
        let expected = vec![
            Ident("let"),
            Ident("x"),
            Symbol("="),
            Symbol("["),
            StringLiteral("\"Hello\""),
            Symbol(","),
            StringLiteral("\'Hello\'"),
            Symbol("]"),
            Symbol(";"),
            StringLiteral("\"\"\" python \"\"\""),
            StringLiteral("''' python '''"),
        ];
        assert_eq!(tokens, expected);
    }

    #[test]
    fn test_unterminated_string_literal() {
        let sample = r#"
            fn main<'a>() {};
        "#;

        let tokens: Vec<Token> = Token::lexer(sample).collect();
        let expected = vec![
            Ident("fn"),
            Ident("main"),
            Symbol("<"),
            StringLiteral("'"),
            Ident("a"),
            Symbol(">"),
            Symbol("("),
            Symbol(")"),
            Symbol("{"),
            Symbol("}"),
            Symbol(";"),
        ];
        assert_eq!(tokens, expected);
    }

    #[test]
    fn test_tokenizer_line_comment() {
        let sample = r#"
        // This is a line comment
        let x
        /// this is also a // line comment
        # python line comment
        -- lua comment
        % MATLAB comment
        "#;

        let tokens: Vec<Token> = Token::lexer(sample).collect();
        let expected = vec![
            LineComment("// This is a line comment"),
            Ident("let"),
            Ident("x"),
            LineComment("/// this is also a // line comment"),
            LineComment("# python line comment"),
            LineComment("-- lua comment"),
            LineComment("% MATLAB comment"),
        ];
        assert_eq!(tokens, expected);
    }

    #[test]
    fn test_tokenizer_block_comment() {
        let sample = r#"
        /* This is a block comment */
        {- Haskell -}
        (* Ocaml *)
        <!-- xml -->
        let x
        /***
* this is a multiline block comment
*/
        "#;

        let tokens: Vec<Token> = Token::lexer(sample).collect();
        let expected = vec![
            BlockComment("/* This is a block comment */"),
            BlockComment("{- Haskell -}"),
            BlockComment("(* Ocaml *)"),
            BlockComment("<!-- xml -->"),
            Ident("let"),
            Ident("x"),
            BlockComment("/***\n* this is a multiline block comment\n*/"),
        ];
        assert_eq!(tokens, expected);
    }

    #[test]
    fn test_open_block_comment() {
        let sample = r#"
          /* hello
          let x
          /*"#;
        let tokens: Vec<Token> = Token::lexer(sample).collect();
        let expected = vec![
            BlockComment("/*"),
            Ident("hello"),
            Ident("let"),
            Ident("x"),
            BlockComment("/*"),
        ];
        assert_eq!(tokens, expected);
    }

    #[test]
    fn test_block_comment_utf8_chars() {
        let sample = r#"/* input:
         *	%rcx: iv (t ⊕ αⁿ ∈ GF(2¹²⁸))
         */"#;
        let tokens: Vec<Token> = Token::lexer(sample).collect();
        let expected = vec![BlockComment(
            "/* input:
         *	%rcx: iv (t ⊕ αⁿ ∈ GF(2¹²⁸))
         */",
        )];
        assert_eq!(tokens, expected);
    }

    #[test]
    fn test_number() {
        let sample = r#"
            x = 1000;
            100_000;
            -1;
            -1.5;
            +1.5;
            -0b01_000;
            -0o700;
            -0xF9f;
            _
            0xFFFFFFFFFFFFFFFFF;
            1000f64;
            (1);
        "#;

        let tokens: Vec<Token> = Token::lexer(sample).collect();
        let expected = vec![
            Ident("x"),
            Symbol("="),
            Number(1000f64),
            Symbol(";"),
            Number(100_000f64),
            Symbol(";"),
            Number(-1f64),
            Symbol(";"),
            Number(-1.5),
            Symbol(";"),
            Number(1.5),
            Symbol(";"),
            Number(-8.0),
            Symbol(";"),
            Number(-448.0),
            Symbol(";"),
            Number(-3999.0),
            Symbol(";"),
            Ident("_"),
            Number(0.0),
            Symbol(";"),
            Ident("1000f64"),
            Symbol(";"),
            Symbol("("),
            Number(1.0),
            Symbol(")"),
            Symbol(";"),
        ];
        assert_eq!(tokens, expected);
    }

    #[test]
    fn test_edge_cases() {
        let sample = r#"
            /* "Hello" */
            " /*Hello*/ "
            "#;
        let tokens: Vec<Token> = Token::lexer(sample).collect();
        let expected = vec![
            BlockComment(r#"/* "Hello" */"#),
            StringLiteral("\" /*Hello*/ \""),
        ];
        assert_eq!(tokens, expected);
    }
}
