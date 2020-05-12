use polyglot_tokenizer::{self, Token};
use pyo3::prelude::*;
use pyo3::wrap_pyfunction;

#[pyfunction]
fn tokenize(content: &str) -> Vec<(&str, String)> {
    polyglot_tokenizer::tokenize(content)
        .map(|token| match token {
            Token::Ident(t) => ("ident", t.to_string()),
            Token::Symbol(t) => ("symbol", t.to_string()),
            Token::LineComment(t) => ("line-comment", t.to_string()),
            Token::BlockComment(t) => ("block-comment", t.to_string()),
            Token::StringLiteral(t) => ("string", t.to_string()),
            Token::Number(t) => ("number", t.to_string()),
            Token::Error => ("error", "".to_string()),
        })
        .collect()
}

#[pymodule]
fn py_polyglot_tokenizer(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_wrapped(wrap_pyfunction!(tokenize))?;

    Ok(())
}
