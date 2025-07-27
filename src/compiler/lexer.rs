use std::{fmt, ops::Range};

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
fn extract_line_doc(lex: &mut Lexer<Token>) -> String {
    lex.slice()[3..].to_owned()
}
#[derive(Logos, Debug, Clone, PartialEq)]
#[logos(error = Range<usize>)]
// single line comments
#[logos(skip r"//[^\n]*")]
// multiline comments
#[logos(skip r"/\*[^*]*\*+(?:[^/*][^*]*\*+)*/")]
// #[logos(skip r"[ \n\t\r\f]+")]
#[rustfmt::skip]

pub enum Token {
    #[regex(r"///[^\n]*", extract_line_doc)]
    LineDoc(String),
    
    // Operators & Separators
    #[token("(")] LeftParen,
    #[token(")")] RightParen,
    #[token("[")] LeftBracket,
    #[token("]")] RightBracket,
    #[token("{")] LeftCurly,
    #[token("}")] RightCurly,
    #[token("<")] LeftAngle,
    #[token(">")] RightAngle,

    #[token("%")] Percent,
    #[token("+")] Plus,
    #[token("-")] Minus,
    #[token("*")] Star,
    #[token("/")] Slash,
    #[token(":")] Colon,
    #[token("::")] DoubleColon,
    #[token(":::")] TripleColon,
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
    #[token("@")] At,
    #[token("&")] Ampersand,
    #[token("|")] Pipe,
    #[token("!")] Bang,
    #[token("`")] Backtick,

    #[token("<=")] LessEqual,
    #[token(">=")] GreaterEqual,
    #[token("!=")] NotStructuralEqual,
    #[token("!==")] NotEqual,
    #[token("==")] StructuralEqual,
    #[token("===")] Equal,
    #[token("is")] Is,

    // Keywords
    #[token("true")] TrueKW,
    #[token("false")] FalseKW,
    #[token("null")] NullKW,

    #[token("?")] PlaceholderKW,
    #[token("val")] ValKW,
    #[token("ref")] RefKW,

    // decimal literals (infinity, nan)
    #[regex(r"[+-]?[Ii]nfinity", allocated_string)] InfinityLiteral(String),
    #[regex(r"[+-]?(?:nan|NaN)")] NanLiteral,

    // Value literals
    // decimal
    // ### Supported formats:
    // - Standard decimals:
    //   - `123.456`
    //   - `0.001`
    //   - `.789`
    //   - `123.`
    //   - `3.e10`
    //   - `534.e-124`
    // - Decimals with exponent:
    //   - `1.23e10`
    //   - `4.56E-3`
    //   - `789e+2`
    //   - `42e0`
    // - Integer with exponent (no decimal point):
    //   - `123e5`
    //   - `42E-1`
    // - Special values:
    //   - `NaN`, `nan`
    //   - `Infinity`, `infinity`
    // - Optional leading sign is supported for all formats:
    //   - `-123.45`, `+123.45`
    //   - `-Infinity`, `+Infinity`
    //   - `-3.e10`, `+3.e10`
    #[regex(r"[+-]?(((0|[1-9])(\d|_)*)?\.(\d|_)+(?:[eE][+-]?(\d|_)+)?|((0|[1-9])(\d|_)*)\.|((0|[1-9])(\d|_)*)[eE][+-]?(\d|_)+)", allocated_string)] DecimalLiteral(String),
    // integer
    // ### Supported formats:
    // - Hexadecimal integers:
    //     - `0x1A2B3C4D5E6F`
    //     - `0X1A2B3C4D5E6F`
    // - Octal integers:
    //     - `0o755`
    //     - `0O755`
    // - Binary integers:
    //     - `0b101010`
    //     - `0B101010`
    // - Decimal integers:
    //     - `123456789`
    //     - `-123456789`
    // - Integers with underscores:
    //     - `1_234_567`
    //     - `-1_234_567`
    // - Decimal integers with leading zeros:
    // - `0123`
    // - `-0123`
    #[regex(r"[+-]?(0|[1-9])(\d|_)*", allocated_string)] IntegerLiteral(String),
    // binary integer
    #[regex(r"0[bB][01_]+", allocated_string)] BinaryIntegerLiteral(String),
    // octal integer
    #[regex(r"0[oO][0-7_]+", allocated_string)] OctalIntegerLiteral(String),
    // hexadecimal integer
    #[regex(r"0[xX][0-9a-fA-F_]+", allocated_string)] HexadecimalIntegerLiteral(String),

    // fraction (e.g. 1/2)
    #[regex(r"[+-]?\d+/\d+", allocated_string)] FractionLiteral(String),

    #[regex(r#"[a-z0-9]*("(?:\\.|[^\\"])*"|'(?:\\.|[^\\'])*')"#, allocated_string)] StringLiteral(String),


    #[regex(r"@[+@]?\w+", allocated_string)] Endpoint(String),

    // identifiers
    #[regex(r"[_\p{L}][_\p{L}\p{N}]*", allocated_string)] Identifier(String),

    // number slots (starting with #, followed by digits)
    #[regex(r"#\d+", allocated_string)] Slot(String),

    // named slots (starting with #, followed by A-Z or a-z)
    #[regex(r"#[_a-zA-Z]+", allocated_string)] NamedSlot(String),

    #[regex(r"[ \t\n\f]")]
    Whitespace,


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
    fn test_integer() {
        let mut lexer = Token::lexer("42");
        assert_eq!(
            lexer.next().unwrap(),
            Ok(Token::IntegerLiteral("42".to_string()))
        );
    }

    #[test]
    fn test_decimal() {
        let mut lexer = Token::lexer("3.14");
        assert_eq!(
            lexer.next().unwrap(),
            Ok(Token::DecimalLiteral("3.14".to_string()))
        );
    }

    #[test]
    fn test_infinity() {
        let mut lexer = Token::lexer("Infinity");
        assert_eq!(
            lexer.next().unwrap(),
            Ok(Token::InfinityLiteral("Infinity".to_string()))
        );

        let mut lexer = Token::lexer("infinity");
        assert_eq!(
            lexer.next().unwrap(),
            Ok(Token::InfinityLiteral("infinity".to_string()))
        );

        let mut lexer = Token::lexer("-Infinity");
        assert_eq!(
            lexer.next().unwrap(),
            Ok(Token::InfinityLiteral("-Infinity".to_string()))
        );

        let mut lexer = Token::lexer("+Infinity");
        assert_eq!(
            lexer.next().unwrap(),
            Ok(Token::InfinityLiteral("+Infinity".to_string()))
        );
    }

    #[test]
    fn test_nan() {
        let mut lexer = Token::lexer("NaN");
        assert_eq!(lexer.next().unwrap(), Ok(Token::NanLiteral));

        let mut lexer = Token::lexer("nan");
        assert_eq!(lexer.next().unwrap(), Ok(Token::NanLiteral));

        let mut lexer = Token::lexer("-NaN");
        assert_eq!(lexer.next().unwrap(), Ok(Token::NanLiteral));

        let mut lexer = Token::lexer("+NaN");
        assert_eq!(lexer.next().unwrap(), Ok(Token::NanLiteral));
    }

    #[test]
    fn test_fraction() {
        let mut lexer = Token::lexer("1/2");
        assert_eq!(
            lexer.next().unwrap(),
            Ok(Token::FractionLiteral("1/2".to_string()))
        );

        let mut lexer = Token::lexer("-3/4");
        assert_eq!(
            lexer.next().unwrap(),
            Ok(Token::FractionLiteral("-3/4".to_string()))
        );

        let mut lexer = Token::lexer("+5/6");
        assert_eq!(
            lexer.next().unwrap(),
            Ok(Token::FractionLiteral("+5/6".to_string()))
        );
    }

    #[test]
    fn test_hexadecimal_integer() {
        let mut lexer = Token::lexer("0x1A3F");
        assert_eq!(
            lexer.next().unwrap(),
            Ok(Token::HexadecimalIntegerLiteral("0x1A3F".to_string()))
        );

        let mut lexer = Token::lexer("0XABC");
        assert_eq!(
            lexer.next().unwrap(),
            Ok(Token::HexadecimalIntegerLiteral("0XABC".to_string()))
        );
    }

    #[test]
    fn test_binary_integer() {
        let mut lexer = Token::lexer("0b1010");
        assert_eq!(
            lexer.next().unwrap(),
            Ok(Token::BinaryIntegerLiteral("0b1010".to_string()))
        );

        let mut lexer = Token::lexer("0B1101");
        assert_eq!(
            lexer.next().unwrap(),
            Ok(Token::BinaryIntegerLiteral("0B1101".to_string()))
        );
    }

    #[test]
    fn test_octal_integer() {
        let mut lexer = Token::lexer("0o755");
        assert_eq!(
            lexer.next().unwrap(),
            Ok(Token::OctalIntegerLiteral("0o755".to_string()))
        );

        let mut lexer = Token::lexer("0O644");
        assert_eq!(
            lexer.next().unwrap(),
            Ok(Token::OctalIntegerLiteral("0O644".to_string()))
        );
    }

    #[test]
    fn test_integers_with_underscores() {
        let mut lexer = Token::lexer("1_000");
        assert_eq!(
            lexer.next().unwrap(),
            Ok(Token::IntegerLiteral("1_000".to_string()))
        );

        let mut lexer = Token::lexer("0xFF_FF_FF");
        assert_eq!(
            lexer.next().unwrap(),
            Ok(Token::HexadecimalIntegerLiteral("0xFF_FF_FF".to_string()))
        );

        let mut lexer = Token::lexer("0b1010_1010");
        assert_eq!(
            lexer.next().unwrap(),
            Ok(Token::BinaryIntegerLiteral("0b1010_1010".to_string()))
        );
    }

    #[test]
    fn test_decimals_with_underscores() {
        let mut lexer = Token::lexer("1_000.123_456");
        assert_eq!(
            lexer.next().unwrap(),
            Ok(Token::DecimalLiteral("1_000.123_456".to_string()))
        );

        let mut lexer = Token::lexer("0.123_456e2");
        assert_eq!(
            lexer.next().unwrap(),
            Ok(Token::DecimalLiteral("0.123_456e2".to_string()))
        );

        let mut lexer = Token::lexer("1.234_567e-8");
        assert_eq!(
            lexer.next().unwrap(),
            Ok(Token::DecimalLiteral("1.234_567e-8".to_string()))
        );
    }

    #[test]
    fn test_add() {
        let mut lexer = Token::lexer("1 + 2");
        assert_eq!(
            lexer.next().unwrap(),
            Ok(Token::IntegerLiteral("1".to_string()))
        );
        assert_eq!(lexer.next().unwrap(), Ok(Token::Whitespace));
        assert_eq!(lexer.next().unwrap(), Ok(Token::Plus));
        assert_eq!(lexer.next().unwrap(), Ok(Token::Whitespace));
        assert_eq!(
            lexer.next().unwrap(),
            Ok(Token::IntegerLiteral("2".to_string()))
        );
        assert_eq!(lexer.next(), None);
    }

    #[test]
    fn test_invalid_add() {
        let mut lexer = Token::lexer("1+2");
        assert_eq!(
            lexer.next().unwrap(),
            Ok(Token::IntegerLiteral("1".to_string()))
        );
        assert_eq!(
            lexer.next().unwrap(),
            Ok(Token::IntegerLiteral("+2".to_string()))
        );
    }

    #[test]
    fn test_invalid_fraction() {
        let mut lexer = Token::lexer("42.4/3");
        assert_eq!(
            lexer.next().unwrap(),
            Ok(Token::DecimalLiteral("42.4".to_string()))
        );
        assert_eq!(lexer.next().unwrap(), Ok(Token::Slash));
        assert_eq!(
            lexer.next().unwrap(),
            Ok(Token::IntegerLiteral("3".to_string()))
        );
    }

    #[test]
    fn test_equality() {
        let mut lexer = Token::lexer("a == b");
        assert_eq!(
            lexer.next().unwrap(),
            Ok(Token::Identifier("a".to_string()))
        );
        assert_eq!(lexer.next().unwrap(), Ok(Token::Whitespace));
        assert_eq!(lexer.next().unwrap(), Ok(Token::StructuralEqual));
        assert_eq!(lexer.next().unwrap(), Ok(Token::Whitespace));
        assert_eq!(
            lexer.next().unwrap(),
            Ok(Token::Identifier("b".to_string()))
        );
        assert_eq!(lexer.next(), None);
    }

    #[test]
    fn test_is_operator() {
        let mut lexer = Token::lexer("a is b");
        assert_eq!(
            lexer.next().unwrap(),
            Ok(Token::Identifier("a".to_string()))
        );
        assert_eq!(lexer.next().unwrap(), Ok(Token::Whitespace));
        assert_eq!(lexer.next().unwrap(), Ok(Token::Is));
        assert_eq!(lexer.next().unwrap(), Ok(Token::Whitespace));
        assert_eq!(
            lexer.next().unwrap(),
            Ok(Token::Identifier("b".to_string()))
        );
        assert_eq!(lexer.next(), None);
    }

    #[test]
    fn test_line_doc() {
        let mut lexer = Token::lexer("/// This is a line doc\n42");
        assert_eq!(
            lexer.next().unwrap(),
            Ok(Token::LineDoc(" This is a line doc".to_string()))
        );
        assert_eq!(lexer.next().unwrap(), Ok(Token::Whitespace));
        assert_eq!(
            lexer.next().unwrap(),
            Ok(Token::IntegerLiteral("42".to_string()))
        );
        assert_eq!(lexer.next(), None);
    }
}
