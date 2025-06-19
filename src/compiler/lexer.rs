use std::fmt;

use logos::{Lexer, Logos};

pub type SourceId = usize;

#[derive(Debug, Clone, PartialEq, Hash)]
pub struct Loc {
    pub source: SourceId,
    pub span: core::ops::Range<usize>,
}

impl Loc {
    pub fn new(source: SourceId, span: core::ops::Range<usize>) -> Self {
        Self { source, span }
    }
}

#[derive(Logos, Debug, Clone, PartialEq)]
#[logos(skip r"//[^\n]*")]
#[logos(skip r"[ \n\t\r\f]+")] // Do not skip newline as it acts as semicolon
#[rustfmt::skip]
pub enum Token {
    // ==< Operators & Separators >==
    #[token("(")] LeftParen,
    #[token(")")] RightParen,
    #[token("[")] LeftBracket,
    #[token("]")] RightBracket,
    #[token("{")] LeftCurly,
    #[token("}")] RightCurly,
    #[token("<")] LeftAngle,
    #[token(">")] RightAngle,

    #[token("*")] Star,
    #[token("/")] Slash,
    #[token("%")] Percent,
    #[token("+")] Plus,
    #[token("-")] Minus,
    #[token(":")] Colon,
    #[token("::")] DoubleColon,
    #[token(";")] Semicolon,
    #[token(".")] Dot,
    #[token(",")] Comma,
    #[token("=")] Assign,

    #[token("++")] Increment,
    #[token("--")] Decrement,
    #[token("&&")] Conjunction,
    #[token("||")] Disjunction,
    #[token("+=")] AddAssign,
    #[token("-=")] SubAssign,
    #[token("*=")] MulAssign,
    #[token("/=")] DivAssign,
    #[token("%=")] ModAssign,

    #[token("->")] Arrow,
    #[token("=>")] FatArrow,
    #[token("..")] Range,
    #[token("..=")] RangeInclusive,
    #[token("...")] Spread,
    #[token("#")] Hash,
    #[token("@")] At,
    #[token("&")] Ampersand,
    #[token("|")] Pipe,
    #[token("!")] Bang,
    #[token("`")] Backtick,

    #[token("<=")] LessEqual,
    #[token(">=")] GreaterEqual,
    #[token("!=")] NotEqual,
    #[token("~=")] AboutEqual, // JS ==
    #[token("==")] EqualEqual, // JS ===

    // ==< Keywords >==
    #[token("async")] AsyncKW,
    #[token("await")] AwaitKW,
    #[token("gen")] GenKW,
    #[token("fn")] FuncKW,
    #[token("struct")] StructKW,
    #[token("enum")] EnumKW,
    #[token("import")] ImportKW,
    #[token("export")] ExportKW,
    #[token("as")] AsKW,
    #[token("yield")] YieldKW,
    #[token("return")] ReturnKW,
    #[token("break")] BreakKW,
    #[token("continue")] ContinueKW,
    #[token("let")] LetKW,
    #[token("loop")] LoopKW,
    #[token("while")] WhileKW,
    #[token("for")] ForKW,
    #[token("if")] IfKW,
    #[token("else")] ElseKW,
    #[token("match")] MatchKW,
    #[token("true")] TrueKW,
    #[token("false")] FalseKW,
    #[token("null")] NullKW,
    #[token("undef")] UndefinedKW,
    #[token("self")] SelfKW,
    #[token("module")] ModuleKW,

    #[token("?")] PlaceholderKW,
    #[token("val")] ValKW,
    #[token("ref")] RefKW,

    // ==< Value literals >==
    #[regex(r"-?(?:0|[1-9]\d+)(?:\.\d+)(?:[eE][+-]?\d+)?", allocated_string)] DecimalLiteral(String),

    #[regex(r"-?(?:0|[1-9]\d*)", allocated_string)] IntegerLiteral(String),

    #[regex(r#"[a-z0-9]*("(?:\\.|[^\\"])*"|'(?:\\.|[^\\'])*')"#, allocated_string)] StringLiteral(String),

    // ==< Other >==
    #[regex(r"[_\p{L}][_\p{L}\p{N}]*", allocated_string)] Identifier(String),

    Error
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}") // Temporary
    }
}

#[inline(always)]
fn allocated_string(lex: &mut Lexer<Token>) -> String {
    lex.slice().to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use logos::Logos;

    #[test]
    fn test_decimal() {
        let mut lexer = Token::lexer("42");
        assert_eq!(lexer.next().unwrap(), Ok(Token::IntegerLiteral("42".to_string())));
    }
}