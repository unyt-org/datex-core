use std::{
    fmt::{self, Display},
    ops::Range,
};

use logos::{Lexer, Logos};

pub type SourceId = usize;

#[derive(Debug, Clone, PartialEq, Hash)]
pub struct Loc {
    pub source: SourceId,
    pub span: core::ops::Range<usize>,
}
use strum::IntoEnumIterator;

use crate::values::core_values::{
    decimal::typed_decimal::DecimalTypeVariant,
    integer::typed_integer::IntegerTypeVariant,
};

impl Loc {
    pub fn new(source: SourceId, span: core::ops::Range<usize>) -> Self {
        Self { source, span }
    }
}
fn extract_line_doc(lex: &mut Lexer<Token>) -> String {
    lex.slice()[3..].to_owned()
}
#[derive(Logos, Debug, Clone, PartialEq, Eq)]
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
    #[regex(r"List *\[")] ListStart,
    #[regex(r"Map *\{")] MapStart,
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
    #[token("!")] Exclamation,
    #[token("`")] Backtick,

    #[token("<=")] LessEqual,
    // #[token(">=")] GreaterEqual, // can not use because of generic overlap type X<test>= 4;
    #[token("!=")] NotStructuralEqual,
    #[token("!==")] NotEqual,
    #[token("==")] StructuralEqual,
    #[token("===")] Equal,
    #[token("is")] Is,
    #[token("matches")] Matches,

    // Keywords
    #[token("true")] True,
    #[token("false")] False,
    #[token("null")] Null,

    #[token("?")] Placeholder,
    #[token("const")] Const,
    #[token("var")] Variable,
    #[token("mut")] Mutable,
    #[token("function")] Function,
    #[token("if")] If,
    #[token("else")] Else,

    // decimal literals (infinity, nan)
    #[regex(r"[+-]?[Ii]nfinity", allocated_string)] Infinity(String),
    #[regex(r"[+-]?(?:nan|NaN)")] Nan,

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
    #[regex(r"[+-]?(((0|[1-9])(\d|_)*)?\.(\d|_)+(?:[eE][+-]?(\d|_)+)?|((0|[1-9])(\d|_)*)\.|((0|[1-9])(\d|_)*)[eE][+-]?(\d|_)+)(?:f32|f64)?", parse_typed_literal::<DecimalTypeVariant>)] DecimalLiteral(DecimalLiteral),
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
    #[regex(r"[+-]?(0|[1-9])(\d|_)*(?:u8|u16|u32|u64|u128|i8|i16|i32|i64|i128|big)?", parse_typed_literal::<IntegerTypeVariant>)] DecimalIntegerLiteral(IntegerLiteral),
    // binary integer
    #[regex(r"0[bB][01_]+*(?:u8|u16|u32|u64|u128|i8|i16|i32|i64|i128|big)?", parse_typed_literal::<IntegerTypeVariant>)] BinaryIntegerLiteral(IntegerLiteral),
    // octal integer
    #[regex(r"0[oO][0-7_]+*(?:u8|u16|u32|u64|u128|i8|i16|i32|i64|i128|big)?", parse_typed_literal::<IntegerTypeVariant>)] OctalIntegerLiteral(IntegerLiteral),
    // hexadecimal integer
    #[regex(r"0[xX][0-9a-fA-F_]+*(?:u8|u16|u32|u64|u128|i8|i16|i32|i64|i128|big)?", parse_typed_literal::<IntegerTypeVariant>)] HexadecimalIntegerLiteral(IntegerLiteral),

    // fraction (e.g. 1/2)
    #[regex(r"[+-]?\d+/\d+", allocated_string)] FractionLiteral(String),

    #[regex(r#"[a-z0-9]*("(?:\\.|[^\\"])*"|'(?:\\.|[^\\'])*')"#, allocated_string)] StringLiteral(String),


    #[regex(r"@[+@]?[a-zA-Z0-9_-]+", allocated_string)] Endpoint(String),

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

impl Token {
    pub fn as_string(&self) -> String {
        let literal_token = match self {
            Token::LeftParen => Some("("),
            Token::RightParen => Some(")"),
            Token::LeftBracket => Some("["),
            Token::RightBracket => Some("]"),
            Token::LeftCurly => Some("{"),
            Token::RightCurly => Some("}"),
            Token::LeftAngle => Some("<"),
            Token::RightAngle => Some(">"),
            Token::Percent => Some("%"),
            Token::Plus => Some("+"),
            Token::Minus => Some("-"),
            Token::Slash => Some("/"),
            Token::Colon => Some(":"),
            Token::DoubleColon => Some("::"),
            Token::TripleColon => Some(":::"),
            Token::Semicolon => Some(";"),
            Token::Dot => Some("."),
            Token::Comma => Some(","),
            Token::Assign => Some("="),
            Token::Increment => Some("++"),
            Token::Decrement => Some("--"),
            Token::Conjunction => Some("&&"),
            Token::Disjunction => Some("||"),
            Token::AddAssign => Some("+="),
            Token::SubAssign => Some("-="),
            Token::MulAssign => Some("*="),
            Token::DivAssign => Some("/="),
            Token::ModAssign => Some("%="),
            Token::Arrow => Some("->"),
            Token::FatArrow => Some("=>"),
            Token::Range => Some(".."),
            Token::RangeInclusive => Some("..="),
            Token::Spread => Some("..."),
            Token::At => Some("@"),
            Token::Ampersand => Some("&"),
            Token::Pipe => Some("|"),
            Token::Backtick => Some("`"),
            Token::LessEqual => Some("<="),
            // Token::GreaterEqual => Some(">="),
            Token::NotStructuralEqual => Some("!="),
            Token::NotEqual => Some("!=="),
            Token::StructuralEqual => Some("=="),
            Token::Equal => Some("==="),
            Token::Is => Some("is"),
            Token::True => Some("true"),
            Token::False => Some("false"),
            Token::Null => Some("null"),
            Token::Placeholder => Some("?"),
            Token::Const => Some("const"),
            Token::Variable => Some("var"),
            Token::Mutable => Some("mut"),
            Token::Function => Some("function"),
            Token::Whitespace => Some(" "),
            Token::Error => Some("error"),
            Token::Infinity(_) => Some("infinity"),
            Token::Nan => Some("nan"),
            Token::Star => Some("*"),
            Token::Exclamation => Some("!"),
            _ => None,
        };
        if let Some(token) = literal_token {
            return format!("'{}'", token);
        }

        let identifier_token = match self {
            Token::LineDoc(_) => "line doc",
            Token::DecimalLiteral(_) => "decimal literal",
            Token::DecimalIntegerLiteral(_) => "decimal integer literal",
            Token::BinaryIntegerLiteral(_) => "binary integer literal",
            Token::OctalIntegerLiteral(_) => "octal integer literal",
            Token::HexadecimalIntegerLiteral(_) => {
                "hexadecimal integer literal"
            }
            Token::FractionLiteral(_) => "fraction literal",
            Token::StringLiteral(_) => "string literal",
            Token::Endpoint(_) => "endpoint",
            Token::Slot(_) => "slot",
            Token::NamedSlot(_) => "named slot",
            Token::Error => "error",
            Token::Identifier(s) => s,
            Token::Matches => "matches",
            Token::If => "if",
            Token::Else => "else",
            e => todo!("Unhandled token in as_string: {:?}", e),
        };

        identifier_token.to_string()
    }
}

pub type IntegerLiteral = TypedLiteral<IntegerTypeVariant>;
pub type DecimalLiteral = TypedLiteral<DecimalTypeVariant>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypedLiteral<T> {
    pub value: String,
    pub variant: Option<T>,
}

impl Display for TypedLiteral<IntegerTypeVariant> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(variant) = &self.variant {
            write!(f, "{}{}", self.value, variant.as_ref())
        } else {
            write!(f, "{}", self.value)
        }
    }
}

trait TypeSuffix: IntoEnumIterator + Copy + AsRef<str> {}
impl<T> TypeSuffix for T where T: IntoEnumIterator + Copy + AsRef<str> {}

fn parse_typed_literal<T: TypeSuffix>(
    lex: &mut Lexer<Token>,
) -> TypedLiteral<T> {
    let mut variant = None;
    let mut number_part = lex.slice();
    for suffix in T::iter() {
        let suffix_str = suffix.as_ref();
        if number_part.ends_with(suffix_str) {
            variant = Some(suffix);
            number_part = &number_part[..number_part.len() - suffix_str.len()];
            break;
        }
    }
    TypedLiteral {
        value: number_part.to_string(),
        variant,
    }
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
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
    fn integer() {
        let mut lexer = Token::lexer("42");
        assert_eq!(
            lexer.next().unwrap(),
            Ok(Token::DecimalIntegerLiteral(IntegerLiteral {
                value: "42".to_string(),
                variant: None
            }))
        );
    }

    #[test]
    fn integer_type() {
        let mut lexer = Token::lexer("42u8");
        let res = lexer.next().unwrap();
        if let Ok(Token::DecimalIntegerLiteral(literal)) = res {
            assert_eq!(literal.value, "42");
            assert_eq!(literal.variant, Some(IntegerTypeVariant::U8));
            assert_eq!(format!("{}", literal), "42u8".to_string());
        } else {
            panic!("Expected DecimalIntegerLiteral with variant U8");
        }

        let mut lexer = Token::lexer("42");
        let res = lexer.next().unwrap();
        if let Ok(Token::DecimalIntegerLiteral(literal)) = res {
            assert_eq!(literal.value, "42");
            assert_eq!(literal.variant, None);
            assert_eq!(format!("{}", literal), "42".to_string());
        } else {
            panic!("Expected DecimalIntegerLiteral with no variant");
        }
    }

    #[test]
    fn integer_with_type() {
        let mut lexer = Token::lexer("42u8");
        assert_eq!(
            lexer.next().unwrap(),
            Ok(Token::DecimalIntegerLiteral(IntegerLiteral {
                value: "42".to_string(),
                variant: Some(IntegerTypeVariant::U8)
            }))
        );

        let mut lexer = Token::lexer("42i32");
        assert_eq!(
            lexer.next().unwrap(),
            Ok(Token::DecimalIntegerLiteral(IntegerLiteral {
                value: "42".to_string(),
                variant: Some(IntegerTypeVariant::I32)
            }))
        );

        let mut lexer = Token::lexer("42big");
        assert_eq!(
            lexer.next().unwrap(),
            Ok(Token::DecimalIntegerLiteral(IntegerLiteral {
                value: "42".to_string(),
                variant: Some(IntegerTypeVariant::Big)
            }))
        );
    }

    #[test]
    fn decimal() {
        let mut lexer = Token::lexer("3.14");
        assert_eq!(
            lexer.next().unwrap(),
            Ok(Token::DecimalLiteral(TypedLiteral::<DecimalTypeVariant> {
                value: "3.14".to_string(),
                variant: None
            }))
        );
    }

    #[test]
    fn decimal_with_type() {
        let mut lexer = Token::lexer("3.14f32");
        assert_eq!(
            lexer.next().unwrap(),
            Ok(Token::DecimalLiteral(TypedLiteral::<DecimalTypeVariant> {
                value: "3.14".to_string(),
                variant: Some(DecimalTypeVariant::F32)
            }))
        );

        let mut lexer = Token::lexer("3.14f64");
        assert_eq!(
            lexer.next().unwrap(),
            Ok(Token::DecimalLiteral(TypedLiteral::<DecimalTypeVariant> {
                value: "3.14".to_string(),
                variant: Some(DecimalTypeVariant::F64)
            }))
        );
    }

    #[test]
    fn infinity() {
        let mut lexer = Token::lexer("Infinity");
        assert_eq!(
            lexer.next().unwrap(),
            Ok(Token::Infinity("Infinity".to_string()))
        );

        let mut lexer = Token::lexer("infinity");
        assert_eq!(
            lexer.next().unwrap(),
            Ok(Token::Infinity("infinity".to_string()))
        );

        let mut lexer = Token::lexer("-Infinity");
        assert_eq!(
            lexer.next().unwrap(),
            Ok(Token::Infinity("-Infinity".to_string()))
        );

        let mut lexer = Token::lexer("+Infinity");
        assert_eq!(
            lexer.next().unwrap(),
            Ok(Token::Infinity("+Infinity".to_string()))
        );
    }

    #[test]
    fn nan() {
        let mut lexer = Token::lexer("NaN");
        assert_eq!(lexer.next().unwrap(), Ok(Token::Nan));

        let mut lexer = Token::lexer("nan");
        assert_eq!(lexer.next().unwrap(), Ok(Token::Nan));

        let mut lexer = Token::lexer("-NaN");
        assert_eq!(lexer.next().unwrap(), Ok(Token::Nan));

        let mut lexer = Token::lexer("+NaN");
        assert_eq!(lexer.next().unwrap(), Ok(Token::Nan));
    }

    #[test]
    fn fraction() {
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
    fn hexadecimal_integer() {
        let mut lexer = Token::lexer("0x1A3F");
        assert_eq!(
            lexer.next().unwrap(),
            Ok(Token::HexadecimalIntegerLiteral(IntegerLiteral {
                value: "0x1A3F".to_string(),
                variant: None
            }))
        );

        let mut lexer = Token::lexer("0XABC");
        assert_eq!(
            lexer.next().unwrap(),
            Ok(Token::HexadecimalIntegerLiteral(IntegerLiteral {
                value: "0XABC".to_string(),
                variant: None
            }))
        );
    }

    #[test]
    fn binary_integer() {
        let mut lexer = Token::lexer("0b1010");
        assert_eq!(
            lexer.next().unwrap(),
            Ok(Token::BinaryIntegerLiteral(IntegerLiteral {
                value: "0b1010".to_string(),
                variant: None
            }))
        );

        let mut lexer = Token::lexer("0B1101");
        assert_eq!(
            lexer.next().unwrap(),
            Ok(Token::BinaryIntegerLiteral(IntegerLiteral {
                value: "0B1101".to_string(),
                variant: None
            }))
        );
    }

    #[test]
    fn octal_integer() {
        let mut lexer = Token::lexer("0o755");
        assert_eq!(
            lexer.next().unwrap(),
            Ok(Token::OctalIntegerLiteral(IntegerLiteral {
                value: "0o755".to_string(),
                variant: None
            }))
        );

        let mut lexer = Token::lexer("0O644");
        assert_eq!(
            lexer.next().unwrap(),
            Ok(Token::OctalIntegerLiteral(IntegerLiteral {
                value: "0O644".to_string(),
                variant: None
            }))
        );
    }

    #[test]
    fn integers_with_underscores() {
        let mut lexer = Token::lexer("1_000");
        assert_eq!(
            lexer.next().unwrap(),
            Ok(Token::DecimalIntegerLiteral(IntegerLiteral {
                value: "1_000".to_string(),
                variant: None
            }))
        );

        let mut lexer = Token::lexer("0xFF_FF_FF");
        assert_eq!(
            lexer.next().unwrap(),
            Ok(Token::HexadecimalIntegerLiteral(IntegerLiteral {
                value: "0xFF_FF_FF".to_string(),
                variant: None
            }))
        );

        let mut lexer = Token::lexer("0b1010_1010");
        assert_eq!(
            lexer.next().unwrap(),
            Ok(Token::BinaryIntegerLiteral(IntegerLiteral {
                value: "0b1010_1010".to_string(),
                variant: None
            }))
        );
    }

    #[test]
    fn decimals_with_underscores() {
        let mut lexer = Token::lexer("1_000.123_456");
        assert_eq!(
            lexer.next().unwrap(),
            Ok(Token::DecimalLiteral(TypedLiteral::<DecimalTypeVariant> {
                value: "1_000.123_456".to_string(),
                variant: None
            }))
        );

        let mut lexer = Token::lexer("0.123_456e2");
        assert_eq!(
            lexer.next().unwrap(),
            Ok(Token::DecimalLiteral(TypedLiteral::<DecimalTypeVariant> {
                value: "0.123_456e2".to_string(),
                variant: None
            }))
        );

        let mut lexer = Token::lexer("1.234_567e-8");
        assert_eq!(
            lexer.next().unwrap(),
            Ok(Token::DecimalLiteral(TypedLiteral::<DecimalTypeVariant> {
                value: "1.234_567e-8".to_string(),
                variant: None
            }))
        );
    }

    #[test]
    fn add() {
        let mut lexer = Token::lexer("1 + 2");
        assert_eq!(
            lexer.next().unwrap(),
            Ok(Token::DecimalIntegerLiteral(IntegerLiteral {
                value: "1".to_string(),
                variant: None
            }))
        );
        assert_eq!(lexer.next().unwrap(), Ok(Token::Whitespace));
        assert_eq!(lexer.next().unwrap(), Ok(Token::Plus));
        assert_eq!(lexer.next().unwrap(), Ok(Token::Whitespace));
        assert_eq!(
            lexer.next().unwrap(),
            Ok(Token::DecimalIntegerLiteral(IntegerLiteral {
                value: "2".to_string(),
                variant: None
            }))
        );
        assert_eq!(lexer.next(), None);
    }

    #[test]
    fn invalid_add() {
        let mut lexer = Token::lexer("1+2");
        assert_eq!(
            lexer.next().unwrap(),
            Ok(Token::DecimalIntegerLiteral(IntegerLiteral {
                value: "1".to_string(),
                variant: None
            }))
        );
        assert_eq!(
            lexer.next().unwrap(),
            Ok(Token::DecimalIntegerLiteral(IntegerLiteral {
                value: "+2".to_string(),
                variant: None
            }))
        );
    }

    #[test]
    fn invalid_fraction() {
        let mut lexer = Token::lexer("42.4/3");
        assert_eq!(
            lexer.next().unwrap(),
            Ok(Token::DecimalLiteral(TypedLiteral::<DecimalTypeVariant> {
                value: "42.4".to_string(),
                variant: None
            }))
        );
        assert_eq!(lexer.next().unwrap(), Ok(Token::Slash));
        assert_eq!(
            lexer.next().unwrap(),
            Ok(Token::DecimalIntegerLiteral(IntegerLiteral {
                value: "3".to_string(),
                variant: None
            }))
        );
    }

    #[test]
    fn equality() {
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
    fn is_operator() {
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
    fn matches_operator() {
        let mut lexer = Token::lexer("a matches b");
        assert_eq!(
            lexer.next().unwrap(),
            Ok(Token::Identifier("a".to_string()))
        );
        assert_eq!(lexer.next().unwrap(), Ok(Token::Whitespace));
        assert_eq!(lexer.next().unwrap(), Ok(Token::Matches));
        assert_eq!(lexer.next().unwrap(), Ok(Token::Whitespace));
        assert_eq!(
            lexer.next().unwrap(),
            Ok(Token::Identifier("b".to_string()))
        );
        assert_eq!(lexer.next(), None);
    }

    #[test]
    fn line_doc() {
        let mut lexer = Token::lexer("/// This is a line doc\n42");
        assert_eq!(
            lexer.next().unwrap(),
            Ok(Token::LineDoc(" This is a line doc".to_string()))
        );
        assert_eq!(lexer.next().unwrap(), Ok(Token::Whitespace));
        assert_eq!(
            lexer.next().unwrap(),
            Ok(Token::DecimalIntegerLiteral(IntegerLiteral {
                value: "42".to_string(),
                variant: None
            }))
        );
        assert_eq!(lexer.next(), None);
    }

    #[test]
    fn divide() {
        let mut lexer = Token::lexer("8 /2");
        assert_eq!(
            lexer.next().unwrap(),
            Ok(Token::DecimalIntegerLiteral(IntegerLiteral {
                value: "8".to_string(),
                variant: None
            }))
        );
        assert_eq!(lexer.next().unwrap(), Ok(Token::Whitespace));
        assert_eq!(lexer.next().unwrap(), Ok(Token::Slash));
        assert_eq!(
            lexer.next().unwrap(),
            Ok(Token::DecimalIntegerLiteral(IntegerLiteral {
                value: "2".to_string(),
                variant: None
            }))
        );
        assert_eq!(lexer.next(), None);
    }
}
