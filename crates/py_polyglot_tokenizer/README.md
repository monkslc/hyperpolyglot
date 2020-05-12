# Py Polyglot Tokenizer
A python interface for [Polyglot Tokenizer](https://github.com/monkslc/hyperpolyglot/tree/master/crates/polyglot_tokenizer)

### Usage
```
from py_polyglot_tokenizer import tokenize
tokens = tokenize("x = 'hello';")
print(tokens)
assert tokens == [('ident', 'x'), ('symbol', '='), ('string', "'hello'"), ('symbol', ';')]
```

### Token Types
**ident**
sequences of alphanumeric characters like variable names and keywords

**symbol**
non alphanumberic character identifiers such as parens, semicolons, equals signs

**string**
string literals

**number**
number literals such as 100, 1.1, 0b10101

**line-comment**
line comments

**block-comment**
block comments

**error**
if there was an error processing a sequence of characters the token type will be error
