use circular_queue::CircularQueue;
use std::{
    collections::VecDeque,
    iter::{DoubleEndedIterator, Peekable},
    str::CharIndices,
};

/// Token is an enum whose variants represent each type of possible Token returned from the
/// [`Tokenizer`]. Block Comments and Strings hold both the start and end indicator for the Tokens.
/// Line Comments hold the open indicator for the Tokens. See below for examples.
///
/// # Examples
/// ```
/// use polyglot_tokenizer::{Token, Tokenizer};
///
/// let content = "/* Block Comment */";
/// let tokens: Vec<Token> = Tokenizer::new(content).tokens().collect();
/// let expected = vec![Token::BlockComment("/*", " Block Comment ", "*/")];
///
/// assert_eq!(tokens, expected);
/// ```
/// ```
/// use polyglot_tokenizer::{Token, Tokenizer};
///
/// let content = "// Line Comment";
/// let tokens: Vec<Token> = Tokenizer::new(content).tokens().collect();
/// let expected = vec![Token::LineComment("//", "Line Comment")];
///
/// assert_eq!(tokens, expected);
/// ```
#[derive(Debug, PartialEq, Eq)]
pub enum Token<'a> {
    BlockComment(&'a str, &'a str, &'a str),
    Ident(&'a str),
    LineComment(&'a str, &'a str),
    Number(&'a str),
    String(&'a str, &'a str, &'a str),
    Symbol(&'a str),
}

/// The tokenizer is responsible for turning content into an iterator of [`Token`].
///
/// # Examples
/// ```
/// use polyglot_tokenizer::{Token, Tokenizer};
///
/// let content = r#"let x = 5;"#;
/// let tokens: Vec<Token> = Tokenizer::new(content).tokens().collect();
/// let expected = vec![
///   Token::Ident("let"),
///   Token::Ident("x"),
///   Token::Symbol("="),
///   Token::Number("5"),
///   Token::Symbol(";"),
/// ];
/// assert_eq!(tokens, expected);
/// ```
pub struct Tokenizer<'a> {
    content: &'a str,
}

impl<'a> Tokenizer<'a> {
    pub fn new(content: &'a str) -> Self {
        Tokenizer { content }
    }

    pub fn tokens(&self) -> Tokens<'a> {
        Tokens {
            backlog: VecDeque::new(),
            chars: self.content.char_indices().peekable(),
            content: self.content,
            current_token_idx: 0,
        }
    }
}

pub struct Tokens<'a> {
    backlog: VecDeque<(usize, char)>,
    chars: Peekable<CharIndices<'a>>,
    content: &'a str,
    current_token_idx: usize,
}

impl<'a> Tokens<'a> {
    fn advance(&mut self) -> Option<(usize, char)> {
        self.next_backlog().or_else(|| self.chars.next())
    }

    fn start_new_token(&mut self) -> Option<char> {
        let (idx, ch) = self.advance()?;
        self.current_token_idx = idx;
        Some(ch)
    }

    fn peek(&mut self) -> Option<(usize, char)> {
        self.peek_backlog().or(self.chars.peek().copied())
    }

    fn next_backlog(&mut self) -> Option<(usize, char)> {
        self.backlog.pop_front()
    }

    fn peek_backlog(&mut self) -> Option<(usize, char)> {
        self.backlog.front().copied()
    }

    fn push_backlog<I>(&mut self, new_chars: I)
    where
        I: Iterator<Item = (usize, char)> + DoubleEndedIterator,
    {
        for ch in new_chars.rev() {
            self.backlog.push_front(ch)
        }
    }

    fn token_start(&self) -> usize {
        self.current_token_idx
    }

    fn eat_whitespace(&mut self) -> usize {
        loop {
            match self.peek() {
                Some((_, ch)) if ch.is_whitespace() => self.advance(),
                Some((idx, _)) => break idx,
                None => break self.content.len(),
            };
        }
    }

    fn eat_non_newline_whitespace(&mut self) -> usize {
        loop {
            match self.peek() {
                Some((idx, ch)) if ch == '\n' || ch == '\r' => {
                    break idx;
                }
                Some((_, ch)) if ch.is_whitespace() => self.advance(),
                Some((idx, _)) => {
                    break idx;
                }
                _ => break self.content.len(),
            };
        }
    }

    fn take_if<F>(&mut self, cond: &mut F) -> usize
    where
        F: FnMut(char) -> bool,
    {
        loop {
            match self.peek() {
                Some((idx, ch)) => {
                    if !cond(ch) {
                        break idx;
                    };
                    self.advance();
                }
                None => break self.content.len(),
            };
        }
    }

    fn take_if_slice<F>(&mut self, cond: &mut F) -> &'a str
    where
        F: FnMut(char) -> bool,
    {
        let end = self.take_if(cond);
        self.slice_from_token_start(end)
    }

    fn slice_from_token_start(&self, end: usize) -> &'a str {
        self.slice(self.token_start(), end)
    }

    fn slice(&self, start: usize, end: usize) -> &'a str {
        &self.content[start..end]
    }

    fn block_comment(
        &mut self,
        start_sequence: &Vec<char>,
        end_sequence: &Vec<char>,
    ) -> Option<Token<'a>> {
        let mut symbol = vec![start_sequence[0]];
        for expected_symbol in start_sequence[1..].into_iter() {
            match self.peek() {
                Some((_, ch)) if ch == *expected_symbol => {
                    symbol.push(ch);
                    self.advance();
                }
                _ => {
                    let token_start = self.token_start();
                    let backlog_chars = symbol[1..]
                        .into_iter()
                        .enumerate()
                        .map(|(idx, ch)| (idx + token_start, *ch));
                    self.push_backlog(backlog_chars);

                    return Some(Token::Symbol(self.slice_from_token_start(token_start + 1)));
                }
            }
        }
        let symbol = self.slice_from_token_start(self.token_start() + symbol.len());
        match self.take_block(self.token_start() + symbol.len(), end_sequence) {
            Ok((content, end_sequence)) => Some(Token::BlockComment(symbol, content, end_sequence)),
            Err(token) => Some(token),
        }
    }

    fn take_block(
        &mut self,
        content_idx: usize,
        end_sequence: &Vec<char>,
    ) -> Result<(&'a str, &'a str), Token<'a>> {
        // start with a random char '@' that won't match the closure check
        let mut prev_chars = CircularQueue::with_capacity(end_sequence.len());
        let mut take_if = |ch| {
            let should_take = prev_chars.iter().eq(end_sequence.iter());
            if should_take {
                prev_chars.push(ch);
            }
            should_take
        };

        let end = self.take_if(&mut take_if);
        if prev_chars.iter().eq(end_sequence.iter()) {
            let end_sequence_start = end - end_sequence.len();
            let content = self.slice(content_idx, end_sequence_start);
            let end_sequence = self.slice(end_sequence_start, end);
            Ok((content, end_sequence))
        } else {
            let backlog_start = self.token_start() + 1;
            let backlog_chars = self
                .slice(backlog_start, end)
                .char_indices()
                .map(|(idx, ch)| (idx + backlog_start, ch));
            self.push_backlog(backlog_chars);
            Err(Token::Symbol(self.slice_from_token_start(backlog_start)))
        }
    }
}

impl<'a> Iterator for Tokens<'a> {
    type Item = Token<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        self.eat_whitespace();
        match self.start_new_token() {
            Some(ch) if ch.is_alphabetic() || ch == '_' => Some(Token::Ident(
                self.take_if_slice(&mut |ch| ch.is_alphanumeric() || ch == '_'),
            )),
            Some('0') => match self.peek() {
                Some((_, 'b')) => {
                    self.advance();
                    Some(Token::Number(self.take_if_slice(&mut |ch| {
                        ch == '1' || ch == '0' || ch == '_'
                    })))
                }
                Some((_, 'o')) => {
                    self.advance();
                    Some(Token::Number(self.take_if_slice(&mut |ch| match ch {
                        '0'..='7' | '_' => true,
                        _ => false,
                    })))
                }
                Some((_, 'x')) => {
                    self.advance();
                    Some(Token::Number(self.take_if_slice(&mut |ch| {
                        ch.is_ascii_hexdigit() || ch == '_'
                    })))
                }
                _ => Some(Token::Number(self.take_if_slice(&mut numeric_closure()))),
            },
            Some(ch) if ch == '-' || ch == '+' => match self.peek() {
                Some((_, ch)) if ch.is_numeric() => {
                    Some(Token::Number(self.take_if_slice(&mut numeric_closure())))
                }
                Some((_, '-')) if ch == '-' => {
                    let symbol = self.take_if_slice(&mut |ch| ch == '-');
                    let comment_start = self.eat_non_newline_whitespace();
                    let comment_end = self.take_if(&mut |ch| ch != '\r' && ch != '\n');
                    let comment = self.slice(comment_start, comment_end);
                    Some(Token::LineComment(symbol, comment))
                }
                _ => Some(Token::Symbol(
                    &self.content[self.token_start()..self.token_start() + 1],
                )),
            },
            Some(ch) if ch.is_numeric() => {
                Some(Token::Number(self.take_if_slice(&mut numeric_closure())))
            }
            Some('/') => match self.peek() {
                Some((_, '/')) => {
                    let symbol = self.take_if_slice(&mut |ch| ch == '/');
                    let comment_start = self.eat_non_newline_whitespace();
                    let comment_end = self.take_if(&mut |ch| ch != '\r' && ch != '\n');
                    let comment = self.slice(comment_start, comment_end);
                    Some(Token::LineComment(symbol, comment))
                }
                Some((_, '*')) => self.block_comment(&vec!['/', '*'], &vec!['*', '/']),
                _ => Some(Token::Symbol(
                    self.slice_from_token_start(self.token_start() + 1),
                )),
            },
            Some('{') => match self.peek() {
                Some((_, '-')) => self.block_comment(&vec!['{', '-'], &vec!['-', '}']),
                _ => Some(Token::Symbol(
                    self.slice_from_token_start(self.token_start() + 1),
                )),
            },
            Some('(') => match self.peek() {
                Some((_, '*')) => self.block_comment(&vec!['(', '*'], &vec!['*', ')']),
                _ => Some(Token::Symbol(
                    self.slice_from_token_start(self.token_start() + 1),
                )),
            },
            Some('<') => self.block_comment(&vec!['<', '!', '-', '-'], &vec!['-', '-', '>']),
            Some('#') => {
                let symbol = self.take_if_slice(&mut |ch| ch == '#');
                let comment_start = self.eat_non_newline_whitespace();
                let comment_end = self.take_if(&mut |ch| ch != '\r' && ch != '\n');
                let comment = self.slice(comment_start, comment_end);
                Some(Token::LineComment(symbol, comment))
            }
            Some('%') => {
                let symbol = self.take_if_slice(&mut |ch| ch == '%');
                let comment_start = self.eat_non_newline_whitespace();
                let comment_end = self.take_if(&mut |ch| ch != '\r' && ch != '\n');
                let comment = self.slice(comment_start, comment_end);
                Some(Token::LineComment(symbol, comment))
            }
            Some(quote_char @ '"') | Some(quote_char @ '\'') | Some(quote_char @ '`') => {
                let symbol = self.take_if_slice(&mut |ch| ch == quote_char);
                match symbol.len() {
                    // If there were only one string identifier, assuume a single line string
                    // This is incorrect for the backtick in JavaScript
                    1 => {
                        let mut is_escaped = false;
                        let mut string_closure = |ch: char| {
                            let should_take = !((ch == quote_char && !is_escaped) || ch == '\n');
                            is_escaped = ch == '\\' && !is_escaped;
                            should_take
                        };
                        let string_end = self.take_if(&mut string_closure);
                        let string_content = self.slice(self.token_start() + 1, string_end);
                        match self.peek() {
                            Some((_, ch)) if ch == quote_char => {
                                self.advance();
                                Some(Token::String(
                                    self.slice_from_token_start(self.token_start() + 1),
                                    string_content,
                                    self.slice(string_end, string_end + 1),
                                ))
                            }
                            _ => {
                                let backlog_start = self.token_start() + 1;
                                let chars_to_backlog = string_content
                                    .char_indices()
                                    .map(|(idx, ch)| (idx + backlog_start, ch));

                                self.push_backlog(chars_to_backlog);
                                Some(Token::Symbol(self.slice_from_token_start(backlog_start)))
                            }
                        }
                    }
                    // Empty String
                    2 => Some(Token::String(
                        self.slice_from_token_start(self.token_start() + 1),
                        "",
                        self.slice(self.token_start() + 1, self.token_start() + 2),
                    )),
                    // If there were more than two quote identifiers, assume a mutli line string,
                    // that ends with the same number of identifiers
                    _ => {
                        let string_indicator = vec![quote_char; symbol.len()];
                        match self.take_block(
                            self.token_start() + string_indicator.len(),
                            &string_indicator,
                        ) {
                            Ok((content, end_indicator)) => Some(Token::String(
                                self.slice_from_token_start(self.token_start() + symbol.len()),
                                content,
                                end_indicator,
                            )),
                            Err(token) => Some(token),
                        }
                    }
                }
            }
            Some(ch) if ch.is_ascii_punctuation() => Some(Token::Symbol(
                &self.content[self.token_start()..self.token_start() + 1],
            )),
            Some(ch) => Some(Token::Symbol(
                self.slice_from_token_start(self.token_start() + ch.len_utf8()),
            )),
            None => None,
        }
    }
}

fn numeric_closure() -> Box<dyn FnMut(char) -> bool> {
    let mut seen_decimal = false;
    Box::new(move |ch| match ch {
        ch if ch.is_numeric() || ch == '_' => true,
        '.' if !seen_decimal => {
            seen_decimal = true;
            true
        }
        _ => false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use Token::*;

    #[test]
    fn idents_symbols() {
        let sample = r#"
        fn main() {
            let x_x2 = 京y;
            let _ = 4;
            println!("{}", x_x2);
        }
        "#;

        let tokenizer = Tokenizer::new(sample);
        let tokens: Vec<Token> = tokenizer.tokens().collect();
        let expected = vec![
            Ident("fn"),
            Ident("main"),
            Symbol("("),
            Symbol(")"),
            Symbol("{"),
            Ident("let"),
            Ident("x_x2"),
            Symbol("="),
            Ident("京y"),
            Symbol(";"),
            Ident("let"),
            Ident("_"),
            Symbol("="),
            Number("4"),
            Symbol(";"),
            Ident("println"),
            Symbol("!"),
            Symbol("("),
            String("\"", "{}", "\""),
            Symbol(","),
            Ident("x_x2"),
            Symbol(")"),
            Symbol(";"),
            Symbol("}"),
        ];

        assert_eq!(tokens, expected)
    }

    #[test]
    fn numbers() {
        let sample = r#"
            1;
            1_000;
            -1;
            -1_000;
            1.5;
            .1.5;
            1.1.4;
            0b1010;
            0o700;
            0xFFFFFFFFFFFFFFFFF;
        "#;

        let tokenizer = Tokenizer::new(sample);
        let tokens: Vec<Token> = tokenizer.tokens().collect();
        let expected = vec![
            Number("1"),
            Symbol(";"),
            Number("1_000"),
            Symbol(";"),
            Number("-1"),
            Symbol(";"),
            Number("-1_000"),
            Symbol(";"),
            Number("1.5"),
            Symbol(";"),
            Symbol("."),
            Number("1.5"),
            Symbol(";"),
            Number("1.1"),
            Symbol("."),
            Number("4"),
            Symbol(";"),
            Number("0b1010"),
            Symbol(";"),
            Number("0o700"),
            Symbol(";"),
            Number("0xFFFFFFFFFFFFFFFFF"),
            Symbol(";"),
        ];

        assert_eq!(tokens, expected)
    }

    #[test]
    fn line_comment() {
        let sample = r#"
            // this is a line comment
            /// this is also one
            //
            --Another line
            ## Python here
            % anotha one
        "#;

        let tokenizer = Tokenizer::new(sample);
        let tokens: Vec<Token> = tokenizer.tokens().collect();
        let expected = vec![
            LineComment("//", "this is a line comment"),
            LineComment("///", "this is also one"),
            LineComment("//", ""),
            LineComment("--", "Another line"),
            LineComment("##", "Python here"),
            LineComment("%", "anotha one"),
        ];

        assert_eq!(tokens, expected)
    }

    #[test]
    fn string() {
        let sample = r#"
          "Hello, World"
          'Heyyy, single quotes'
          `Back ticks`
        "#;

        let tokenizer = Tokenizer::new(sample);
        let tokens: Vec<Token> = tokenizer.tokens().collect();
        let expected = vec![
            String("\"", "Hello, World", "\""),
            String("'", "Heyyy, single quotes", "'"),
            String("`", "Back ticks", "`"),
        ];

        assert_eq!(tokens, expected)
    }

    #[test]
    fn string_multiline() {
        let sample = r#"
        """ Hey there
        this is a multiliner"""
        "#;

        let tokenizer = Tokenizer::new(sample);
        let tokens: Vec<Token> = tokenizer.tokens().collect();
        let expected = vec![String(
            "\"\"\"",
            " Hey there\n        this is a multiliner",
            "\"\"\"",
        )];

        assert_eq!(tokens, expected);
    }

    #[test]
    fn string_multiline_other() {
        let sample = r#"
        ''' hey single quotes '''
        ``` hey backticks ```
        "#;

        let tokenizer = Tokenizer::new(sample);
        let tokens: Vec<Token> = tokenizer.tokens().collect();
        let expected = vec![
            String("'''", " hey single quotes ", "'''"),
            String("```", " hey backticks ", "```"),
        ];

        assert_eq!(tokens, expected);
    }

    #[test]
    fn string_unterminated_multiline() {
        let sample = r#"
        """
        "#;

        let tokenizer = Tokenizer::new(sample);
        let tokens: Vec<Token> = tokenizer.tokens().collect();
        let expected = vec![Symbol("\""), String("\"", "", "\"")];

        assert_eq!(tokens, expected);
    }

    #[test]
    fn incomplete_string() {
        let sample = r#"
          "Hello
          10
        "#;

        let tokenizer = Tokenizer::new(sample);
        let tokens: Vec<Token> = tokenizer.tokens().collect();
        let expected = vec![Symbol("\""), Ident("Hello"), Number("10")];

        assert_eq!(tokens, expected)
    }

    #[test]
    fn escaped_quote() {
        let sample = r#"
          "Hello\" World"
        "#;

        let tokenizer = Tokenizer::new(sample);
        let tokens: Vec<Token> = tokenizer.tokens().collect();
        let expected = vec![String("\"", "Hello\\\" World", "\"")];

        assert_eq!(tokens, expected)
    }

    #[test]
    fn misamtched_string_identifiers() {
        let sample = r#"
          "Hello World'
        "#;

        let tokenizer = Tokenizer::new(sample);
        let tokens: Vec<Token> = tokenizer.tokens().collect();
        let expected = vec![Symbol("\""), Ident("Hello"), Ident("World"), Symbol("'")];

        assert_eq!(tokens, expected)
    }

    #[test]
    fn block_comment() {
        let sample = r#"
        /* Comment Here */
        /*    */
        /**/
        /*
         * Multi line*/
        "#;

        let tokenizer = Tokenizer::new(sample);
        let tokens: Vec<Token> = tokenizer.tokens().collect();
        let expected = vec![
            BlockComment("/*", " Comment Here ", "*/"),
            BlockComment("/*", "    ", "*/"),
            BlockComment("/*", "", "*/"),
            BlockComment("/*", "\n         * Multi line", "*/"),
        ];

        assert_eq!(tokens, expected);
    }

    #[test]
    fn other_block_comments() {
        let sample = r#"
        {-comment-}
        (*block*)
        "#;

        let tokenizer = Tokenizer::new(sample);
        let tokens: Vec<Token> = tokenizer.tokens().collect();
        let expected = vec![
            BlockComment("{-", "comment", "-}"),
            BlockComment("(*", "block", "*)"),
        ];

        assert_eq!(tokens, expected);
    }

    #[test]
    fn html_comment() {
        let sample = r#"
        <!-- Comment Here-->
        <!-- 
         Multi line
         Comment
         -->
         <!---->
         <!--       -->
        "#;

        let tokenizer = Tokenizer::new(sample);
        let tokens: Vec<Token> = tokenizer.tokens().collect();
        let expected = vec![
            BlockComment("<!--", " Comment Here", "-->"),
            BlockComment(
                "<!--",
                " \n         Multi line\n         Comment\n         ",
                "-->",
            ),
            BlockComment("<!--", "", "-->"),
            BlockComment("<!--", "       ", "-->"),
        ];

        assert_eq!(tokens, expected);
    }

    #[test]
    fn unterminated_html_comment() {
        let sample = r#"
          <!-- hey
        "#;

        let tokenizer = Tokenizer::new(sample);
        let tokens: Vec<Token> = tokenizer.tokens().collect();
        let expected = vec![Symbol("<"), Symbol("!"), LineComment("--", "hey")];

        assert_eq!(tokens, expected);
    }

    #[test]
    fn unterminated_html_comment2() {
        let sample = r#"
          < let x
        "#;

        let tokenizer = Tokenizer::new(sample);
        let tokens: Vec<Token> = tokenizer.tokens().collect();
        let expected = vec![Symbol("<"), Ident("let"), Ident("x")];

        assert_eq!(tokens, expected);
    }

    #[test]
    fn unterminated_html_comment3() {
        let sample = r#"<"#;

        let tokenizer = Tokenizer::new(sample);
        let tokens: Vec<Token> = tokenizer.tokens().collect();
        let expected = vec![Symbol("<")];

        assert_eq!(tokens, expected);
    }

    #[test]
    fn unterminated_block_comment() {
        let sample = r#"
        /* let x
        "#;

        let tokenizer = Tokenizer::new(sample);
        let tokens: Vec<Token> = tokenizer.tokens().collect();
        let expected = vec![Symbol("/"), Symbol("*"), Ident("let"), Ident("x")];

        assert_eq!(tokens, expected);
    }

    #[test]
    fn random_chars() {
        let sample = r#"
            →
"#;

        let tokenizer = Tokenizer::new(sample);
        let tokens: Vec<Token> = tokenizer.tokens().collect();
        let expected = vec![Symbol("→")];

        assert_eq!(tokens, expected);
    }

    #[test]
    fn nested_backlog() {
        let sample = r#"
        /* `helloworldwhat
         let x = 5
        "#;

        let tokenizer = Tokenizer::new(sample);
        let tokens: Vec<Token> = tokenizer.tokens().collect();
        let expected = vec![
            Symbol("/"),
            Symbol("*"),
            Symbol("`"),
            Ident("helloworldwhat"),
            Ident("let"),
            Ident("x"),
            Symbol("="),
            Number("5"),
        ];

        assert_eq!(tokens, expected);
    }

    #[test]
    fn test_escaped_string() {
        let sample = r#"
          "Hello \"World"
          "Hello World\\"
          "Hello World\" x
        "#;
        let tokens: Vec<_> = Tokenizer::new(sample).tokens().collect();
        let expected = vec![
            String("\"", "Hello \\\"World", "\""),
            String("\"", "Hello World\\\\", "\""),
            Symbol("\""),
            Ident("Hello"),
            Ident("World"),
            Symbol("\\"),
            Symbol("\""),
            Ident("x"),
        ];
        assert_eq!(tokens, expected);
    }
}
