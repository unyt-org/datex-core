use std::fmt;

use crate::compiler::lexer::Token;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Pattern {
    Char(char),
    Any,
    Token(Token),
    Literal,
    EndOfInput,
    SomethingElse,
}

impl From<char> for Pattern {
    fn from(c: char) -> Self {
        Self::Char(c)
    }
}
// impl From<Token> for Pattern {
//     fn from(tok: Token) -> Self {
//         Self::Token(tok)
//     }
// }
// impl From<&Token> for Pattern {
//     fn from(tok: &Token) -> Self {
//         Self::Token(tok.clone())
//     }
// }

impl fmt::Display for Pattern {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Pattern::SomethingElse => write!(f, "something else"),
            Pattern::Any => write!(f, "any"),
            Pattern::Token(token) => write!(f, "{}", token),
            Pattern::Char(c) => write!(f, "{:?}", c),
            Pattern::Literal => write!(f, "literal"),
            Pattern::EndOfInput => write!(f, "end of input"),
        }
    }
}
impl Pattern {
    pub fn kind(&self) -> &'static str {
        match self {
            Pattern::SomethingElse => "token",
            Pattern::Any => "token",
            Pattern::Token(_) => "token",
            Pattern::Char(_) => "character",
            Pattern::Literal => "literal",
            Pattern::EndOfInput => "end of input",
        }
    }
    pub fn as_string(&self) -> String {
        match self {
            Pattern::SomethingElse => "something else".to_string(),
            Pattern::Any => "any".to_string(),
            Pattern::Token(token) => token.as_string(),
            Pattern::Char(c) => format!("'{:?}'", c),
            Pattern::Literal => "literal".to_string(),
            Pattern::EndOfInput => "end of input".to_string(),
        }
    }
}
