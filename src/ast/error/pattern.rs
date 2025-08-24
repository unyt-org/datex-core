use std::fmt;

use crate::compiler::lexer::Token;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Pattern {
    Char(char),
    Token(Token),
    Literal,
    EndOfInput,
}

impl From<char> for Pattern {
    fn from(c: char) -> Self {
        Self::Char(c)
    }
}
impl From<Token> for Pattern {
    fn from(tok: Token) -> Self {
        Self::Token(tok)
    }
}

impl fmt::Display for Pattern {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Pattern::Token(token) => write!(f, "{}", token),
            Pattern::Char(c) => write!(f, "{:?}", c),
            Pattern::Literal => write!(f, "literal"),
            Pattern::EndOfInput => write!(f, "end of input"),
        }
    }
}
