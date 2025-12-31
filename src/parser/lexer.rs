use core::{
    fmt::{self, Display},
    ops::Range,
};

use logos::{Lexer, Logos};

pub type SourceId = usize;

#[derive(Debug, Clone, PartialEq, Hash)]
pub struct Loc {
    pub source: SourceId,
    pub span: Range<usize>,
}
use crate::parser::errors::{ParserError, SpannedParserError};
use crate::values::core_values::{
    decimal::typed_decimal::DecimalTypeVariant,
    integer::typed_integer::IntegerTypeVariant,
};
use strum::IntoEnumIterator;
use strum::IntoEnumIterator;

impl Loc {
    pub fn new(source: SourceId, span: core::ops::Range<usize>) -> Self {
        Self { source, span }
    }
}
fn extract_line_doc(lex: &mut Lexer<Token>) -> String {
    lex.slice()[3..].to_owned()
}

/// ### Supported formats:
/// - Standard decimals:
///   - `123.456`
///   - `0.001`
///   - `.789`
///   - `123.`
///   - `3.e10`
//   - `534.e-124`
/// - Decimals with exponent:
///   - `1.23e10`
///   - `4.56E-3`
///   - `789e+2`
///   - `42e0`
/// - Integer with exponent (no decimal point):
///   - `123e5`
///   - `42E-1`
/// - Special values:
///   - `NaN`, `nan`
///   - `Infinity`, `infinity`
/// - Optional leading sign is supported for all formats:
///   - `-123.45`, `+123.45`
///   - `-Infinity`, `+Infinity`
///   - `-3.e10`, `+3.e10`

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NumericLiteralParts {
    pub integer_part: String,
    pub exponent_part: Option<String>,
    pub variant_part: Option<String>,
}
impl Display for NumericLiteralParts {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        core::write!(f, "{}", self.integer_part)?;
        if let Some(exp) = &self.exponent_part {
            core::write!(f, "e{}", exp)?;
        }
        if let Some(var) = &self.variant_part {
            core::write!(f, "{}", var)?;
        }
        Ok(())
    }
}
impl From<&str> for NumericLiteralParts {
    fn from(value: &str) -> Self {
        NumericLiteralParts::from(value.to_string())
    }
}
impl From<String> for NumericLiteralParts {
    fn from(mut value: String) -> Self {
        value = value.replace('_', "");
        let chars: Vec<char> = value.chars().collect();
        let mut i = 0;

        // 2. Parse integer part
        let start = i;
        while i < chars.len() && chars[i].is_ascii_digit() {
            i += 1;
        }
        let mut integer_part = value[start..i].to_string();

        // 3. Parse fractional part
        if i < chars.len() && chars[i] == '.' {
            i += 1; // skip dot
            let frac_start = i;
            while i < chars.len() && chars[i].is_ascii_digit() {
                i += 1;
            }
            integer_part = value[frac_start..i].to_string();
        }

        // 4. Parse exponent
        let mut exponent_part = None;
        if i < chars.len() && (chars[i] == 'e' || chars[i] == 'E') {
            let exp_start = i;
            i += 1; // consume e/E

            // optional sign
            if i < chars.len() && (chars[i] == '+' || chars[i] == '-') {
                i += 1;
            }

            let digits_start = i;
            while i < chars.len() && chars[i].is_ascii_digit() {
                i += 1;
            }

            // Only accept exponent if it had at least one digit
            if digits_start < i {
                exponent_part = Some(value[exp_start + 1..i].to_string());
            } else {
                // invalid exponent → treat `e` as suffix instead
                i = exp_start;
            }
        }

        // 5. Remaining chars = suffix (any characters)
        let variant_part = if i < chars.len() {
            Some(value[i..].to_string())
        } else {
            None
        };
        NumericLiteralParts {
            exponent_part,
            integer_part,
            variant_part,
        }
    }
}

#[derive(Logos, Debug, Clone, PartialEq, Eq)]
#[logos(error = Range<usize>)]
// single line comments
#[logos(skip r"//[^\n]*")]
// multiline comments
#[logos(skip r"/\*[^*]*\*+(?:[^/*][^*]*\*+)*/")]
// whitespace
#[logos(skip r"[ \n\t\r\f]+")]
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
    #[token("^")] Caret,
    #[token("/")] Slash,
    #[token(":")] Colon,
    #[token("::")] DoubleColon,
    #[token(":::")] TripleColon,
    #[token(";")] Semicolon,
    #[token(",")] Comma,
    #[token("=")] Assign,

    #[token("++")] Increment,
    #[token("--")] Decrement,
    #[token("&&")] DoubleAnd,
    #[token("||")] DoublePipe,
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
    #[token(">=")] GreaterEqual, // can not use because of generic overlap type X<test>= 4;
    #[token("!=")] NotStructuralEqual,
    #[token("!==")] NotEqual,
    #[token("==")] StructuralEqual,
    #[token("===")] Equal,
    #[token("is")] Is,
    #[token("matches")] Matches,
    #[token("and")] And,
    #[token("or")] Or,

    // Keywords
    #[token("true")] True,
    #[token("false")] False,
    #[token("null")] Null,

    #[token("?")] Placeholder,
    #[token("const")] Const,
    #[token("var")] Variable,
    #[token("mut")] Mutable,
    #[token("&mut")] MutRef,
    #[token("function")] Function,
    #[token("if")] If,
    #[token("else")] Else,

    #[token("type")] TypeDeclaration,
    #[token("type(")] TypeExpressionStart,
    #[token("typealias")] TypeAlias,

    #[token(".")]
    Dot,
    // pointer address (e.g. $1234ab, exactly 3, 5 or 26 bytes)
    #[regex(r"\$(?:[0-9a-fA-F]{6}|[0-9a-fA-F]{10}|[0-9a-fA-F]{52})", allocated_string)] PointerAddress(String),

    // decimal literals (infinity, nan)
    #[regex(r"[Ii]nfinity")] Infinity,
    #[regex(r"(?:nan|NaN)")] Nan,

    /// Decimal integer with suffix
    /// Includes
    /// - standard decimal integers (e.g. 1234)
    /// - integers with integer suffix (e.g. 42u8, 100i32, 999ibig)
    /// - integers with exponent (e.g. 12e4, 3E-2) which are treated as decimals
    /// - integers with decimal suffix (e.g. 12f32, 1e4f64) which are treated as decimals
    #[regex(r"\d+[_\d]*(?:[eE][+-]?\d+[_\d]*)?(?:u8|u16|u32|u64|u128|i8|i16|i32|i64|i128|ubig|ibig|f32|f64|dbig)?", allocated_string)]
    IntegerLiteral(String),

    #[regex(
        r"0[bB][01][01_]*(?:u8|u16|u32|u64|u128|i8|i16|i32|i64|i128|ubig|ibig)?",
        parse_typed_literal::<IntegerTypeVariant>
    )]
    BinaryIntegerLiteral(IntegerWithVariant),

    #[regex(
        r"0[oO][0-7][0-7_]*(?:u8|u16|u32|u64|u128|i8|i16|i32|i64|i128|ubig|ibig)?",
        parse_typed_literal::<IntegerTypeVariant>
    )]
    OctalIntegerLiteral(IntegerWithVariant),

    #[regex(
        r"0[xX][0-9a-fA-F][0-9a-fA-F_]*(?:u8|u16|u32|u64|u128|i8|i16|i32|i64|i128|ubig|ibig)?",
        parse_typed_literal::<IntegerTypeVariant>
    )]
    HexadecimalIntegerLiteral(IntegerWithVariant),

    /// Decimal literal (excluding decimals without dot (e.g. 12f32, 1e4) - these are handled by IntegerLiteral)
    #[regex(r"\d+[_\d]*\.[_\d]+(?:[eE][+-]?\d+[_\d]*)?(?:f32|f64|dbig)?", parse_typed_literal::<DecimalTypeVariant>)]
    DecimalLiteral(LiteralWithVariant<DecimalTypeVariant>),

    /// Decimal fraction (e.g. 1/2, 3/4)
    #[regex(r"\d+[_\d]*/\d+[_\d]*", allocated_string)]
    FractionLiteral(String),

    #[regex(r#"[a-z0-9]*("(?:\\.|[^\\"])*"|'(?:\\.|[^\\'])*')"#, allocated_string)] StringLiteral(String),


    #[regex(r"@[+@]?[a-zA-Z0-9_-]+", allocated_string)] Endpoint(String),

    // identifiers
    #[regex(r"[_\p{L}][_\p{L}\p{N}]*", allocated_string, priority=1)] Identifier(String),

    // number slots (starting with #, followed by digits)
    #[regex(r"#\d+", allocated_string)] Slot(String),

    // named slots (starting with #, followed by A-Z, a-z, _ and alphanumeric characters)
    #[regex(r"#[_a-zA-Z][_a-zA-Z0-9]*", allocated_string)] NamedSlot(String),
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
            Token::DoubleAnd => Some("&&"),
            Token::DoublePipe => Some("||"),
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
            Token::GreaterEqual => Some(">="),
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
            Token::Infinity => Some("infinity"),
            Token::Nan => Some("nan"),
            Token::TypeDeclaration => Some("type"),
            Token::TypeExpressionStart => Some("type("),
            Token::TypeAlias => Some("typealias"),
            Token::MutRef => Some("&mut"),
            Token::And => Some("and"),
            Token::Or => Some("or"),
            Token::Star => Some("*"),
            Token::Exclamation => Some("!"),
            Token::Caret => Some("^"),
            _ => None,
        };
        if let Some(token) = literal_token {
            return format!("'{}'", token);
        }

        let identifier_token = match self {
            Token::LineDoc(_) => "line doc",
            // Token::DecimalLiteral(_) => "decimal literal",
            Token::BinaryIntegerLiteral(_) => "binary integer literal",
            Token::OctalIntegerLiteral(_) => "octal integer literal",
            Token::HexadecimalIntegerLiteral(_) => {
                "hexadecimal integer literal"
            }
            Token::StringLiteral(_) => "string literal",
            Token::Endpoint(_) => "endpoint",
            Token::Slot(_) => "slot",
            Token::NamedSlot(_) => "named slot",
            Token::Identifier(s) => s,
            Token::Matches => "matches",
            Token::If => "if",
            Token::Else => "else",
            e => core::todo!("#367 Unhandled token in as_string: {:?}", e),
        };

        identifier_token.to_string()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpannedToken {
    pub token: Token,
    pub span: Range<usize>,
}

pub fn get_spanned_tokens_from_source(
    src: &str,
) -> (Vec<SpannedToken>, Vec<SpannedParserError>) {
    let lexer = Token::lexer(src);
    let (oks, errs): (Vec<_>, Vec<_>) = lexer
        .spanned()
        .map(|(tok, span)| {
            tok.map(|token| SpannedToken { token, span })
                .map_err(|span| SpannedParserError {
                    error: ParserError::InvalidToken,
                    span,
                })
        })
        .partition(Result::is_ok);

    let tokens = oks.into_iter().map(Result::unwrap).collect();
    let errors = errs.into_iter().map(Result::unwrap_err).collect();

    (tokens, errors)
}

pub type IntegerWithVariant = LiteralWithVariant<IntegerTypeVariant>;
pub type DecimalWithVariant = LiteralWithVariant<DecimalTypeVariant>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiteralWithVariant<T> {
    pub value: String,
    pub variant: Option<T>,
}

impl Display for LiteralWithVariant<IntegerTypeVariant> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(variant) = &self.variant {
            core::write!(f, "{}{}", self.value, variant.as_ref())
        } else {
            core::write!(f, "{}", self.value)
        }
    }
}

trait TypeSuffix: IntoEnumIterator + Copy + AsRef<str> {}
impl<T> TypeSuffix for T where T: IntoEnumIterator + Copy + AsRef<str> {}

fn parse_typed_literal<T: TypeSuffix>(
    lex: &mut Lexer<Token>,
) -> LiteralWithVariant<T> {
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
    LiteralWithVariant {
        value: number_part.to_string(),
        variant,
    }
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        core::write!(f, "{self:?}")
    }
}

#[inline(always)]
fn allocated_string(lex: &mut Lexer<Token>) -> String {
    lex.slice().to_owned()
}

fn numeric_parts(lex: &mut Lexer<Token>) -> NumericLiteralParts {
    // 1. Remove underscores
    let s = lex.slice().replace('_', "");
    NumericLiteralParts::from(s)
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
            Ok(Token::IntegerLiteral("42".to_string()))
        );
    }

    #[test]
    fn integer_type() {
        let mut lexer = Token::lexer("42u8");
        let res = lexer.next().unwrap();
        if let Ok(Token::IntegerLiteral(literal)) = res {
            assert_eq!(literal, "42u8".to_string());
            assert_eq!(format!("{}", literal), "42u8".to_string());
        } else {
            core::panic!("Expected DecimalIntegerLiteral with variant U8");
        }

        let mut lexer = Token::lexer("42");
        let res = lexer.next().unwrap();
        if let Ok(Token::IntegerLiteral(literal)) = res {
            assert_eq!(literal, "42".to_string());
        } else {
            core::panic!("Expected DecimalIntegerLiteral with no variant");
        }
    }

    #[test]
    fn integer_with_type() {
        let mut lexer = Token::lexer("42u8");
        assert_eq!(
            lexer.next().unwrap(),
            Ok(Token::IntegerLiteral("42u8".to_string()))
        );

        let mut lexer = Token::lexer("42i32");
        assert_eq!(
            lexer.next().unwrap(),
            Ok(Token::IntegerLiteral("42i32".to_string()))
        );

        let mut lexer = Token::lexer("42ibig");
        assert_eq!(
            lexer.next().unwrap(),
            Ok(Token::IntegerLiteral("42ibig".to_string()))
        );
    }

    #[test]
    fn decimal() {
        let mut lexer = Token::lexer("3.14");
        assert_eq!(
            lexer.next().unwrap(),
            Ok(Token::DecimalLiteral(DecimalWithVariant {
                value: "3.14".to_string(),
                variant: None
            }))
        );
    }

    // #[test]
    // fn decimal_with_type() {
    //     let mut lexer = Token::lexer("3.14f32");
    //     assert_eq!(
    //         lexer.next().unwrap(),
    //         Ok(Token::DecimalLiteralWithSuffix(TypedLiteral::<
    //             DecimalTypeVariant,
    //         > {
    //             value: "3.14".to_string(),
    //             variant: Some(DecimalTypeVariant::F32)
    //         }))
    //     );

    //     let mut lexer = Token::lexer("3.14f64");
    //     assert_eq!(
    //         lexer.next().unwrap(),
    //         Ok(Token::DecimalLiteralWithSuffix(TypedLiteral::<
    //             DecimalTypeVariant,
    //         > {
    //             value: "3.14".to_string(),
    //             variant: Some(DecimalTypeVariant::F64)
    //         }))
    //     );
    // }

    #[test]
    fn infinity() {
        let mut lexer = Token::lexer("Infinity");
        assert_eq!(lexer.next().unwrap(), Ok(Token::Infinity));

        let mut lexer = Token::lexer("infinity");
        assert_eq!(lexer.next().unwrap(), Ok(Token::Infinity));

        let lexer = Token::lexer("-Infinity");
        assert_eq!(
            lexer.map(Result::unwrap).collect::<Vec<_>>(),
            vec![Token::Minus, Token::Infinity]
        );

        let lexer = Token::lexer("+Infinity");
        assert_eq!(
            lexer.map(Result::unwrap).collect::<Vec<_>>(),
            vec![Token::Plus, Token::Infinity]
        );
    }

    #[test]
    fn nan() {
        let mut lexer = Token::lexer("NaN");
        assert_eq!(lexer.next().unwrap(), Ok(Token::Nan));

        let mut lexer = Token::lexer("nan");
        assert_eq!(lexer.next().unwrap(), Ok(Token::Nan));

        let lexer = Token::lexer("-NaN");
        assert_eq!(
            lexer.map(Result::unwrap).collect::<Vec<_>>(),
            vec![Token::Minus, Token::Nan]
        );

        let lexer = Token::lexer("+NaN");
        assert_eq!(
            lexer.map(Result::unwrap).collect::<Vec<_>>(),
            vec![Token::Plus, Token::Nan]
        );
    }

    #[test]
    fn fraction() {
        let mut lexer = Token::lexer("1/2");
        assert_eq!(
            lexer.next().unwrap(),
            Ok(Token::FractionLiteral("1/2".to_string()))
        );

        let mut lexer = Token::lexer("3/4");
        assert_eq!(
            lexer.next().unwrap(),
            Ok(Token::FractionLiteral("3/4".to_string()))
        );

        let mut lexer = Token::lexer("51_11/6");
        assert_eq!(
            lexer.next().unwrap(),
            Ok(Token::FractionLiteral("51_11/6".to_string()))
        );
    }

    #[test]
    fn hexadecimal_integer() {
        let mut lexer = Token::lexer("0x1A3F");
        assert_eq!(
            lexer.next().unwrap(),
            Ok(Token::HexadecimalIntegerLiteral(IntegerWithVariant {
                value: "0x1A3F".to_string(),
                variant: None
            }))
        );

        let mut lexer = Token::lexer("0XABC");
        assert_eq!(
            lexer.next().unwrap(),
            Ok(Token::HexadecimalIntegerLiteral(IntegerWithVariant {
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
            Ok(Token::BinaryIntegerLiteral(IntegerWithVariant {
                value: "0b1010".to_string(),
                variant: None
            }))
        );

        let mut lexer = Token::lexer("0B1101");
        assert_eq!(
            lexer.next().unwrap(),
            Ok(Token::BinaryIntegerLiteral(IntegerWithVariant {
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
            Ok(Token::OctalIntegerLiteral(IntegerWithVariant {
                value: "0o755".to_string(),
                variant: None
            }))
        );

        let mut lexer = Token::lexer("0O644");
        assert_eq!(
            lexer.next().unwrap(),
            Ok(Token::OctalIntegerLiteral(IntegerWithVariant {
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
            Ok(Token::IntegerLiteral("1_000".to_string()))
        );

        let mut lexer = Token::lexer("0xFF_FF_FF");
        assert_eq!(
            lexer.next().unwrap(),
            Ok(Token::HexadecimalIntegerLiteral(IntegerWithVariant {
                value: "0xFF_FF_FF".to_string(),
                variant: None
            }))
        );

        let mut lexer = Token::lexer("0b1010_1010");
        assert_eq!(
            lexer.next().unwrap(),
            Ok(Token::BinaryIntegerLiteral(IntegerWithVariant {
                value: "0b1010_1010".to_string(),
                variant: None
            }))
        );
    }

    #[test]
    fn decimals() {
        // exponent, variant
        let mut lexer = Token::lexer("10.234_567e-8f32");
        assert_eq!(
            lexer.next().unwrap(),
            Ok(Token::DecimalLiteral(DecimalWithVariant {
                value: "10.234_567e-8".to_string(),
                variant: Some(DecimalTypeVariant::F32)
            }))
        );

        // exponent, no variant
        let mut lexer = Token::lexer("10.234_567e-8");
        assert_eq!(
            lexer.next().unwrap(),
            Ok(Token::DecimalLiteral(DecimalWithVariant {
                value: "10.234_567e-8".to_string(),
                variant: None
            }))
        );

        // no exponent, no variant
        let mut lexer = Token::lexer("0.123_456");
        assert_eq!(
            lexer.next().unwrap(),
            Ok(Token::DecimalLiteral(DecimalWithVariant {
                value: "0.123_456".to_string(),
                variant: None
            }))
        );

        // no exponent, variant
        let mut lexer = Token::lexer("1_000.123_456f32");
        assert_eq!(
            lexer.next().unwrap(),
            Ok(Token::DecimalLiteral(DecimalWithVariant {
                value: "1_000.123_456".to_string(),
                variant: Some(DecimalTypeVariant::F32)
            }))
        );
    }

    #[test]
    fn add() {
        let mut lexer = Token::lexer("1 + 2");
        assert_eq!(
            lexer.next().unwrap(),
            Ok(Token::IntegerLiteral("1".to_string()))
        );
        assert_eq!(lexer.next().unwrap(), Ok(Token::Plus));
        assert_eq!(
            lexer.next().unwrap(),
            Ok(Token::IntegerLiteral("2".to_string()))
        );
        assert_eq!(lexer.next(), None);
    }

    #[test]
    fn invalid_fraction() {
        let mut lexer = Token::lexer("42.4/3");
        assert_eq!(
            lexer.next().unwrap(),
            Ok(Token::DecimalLiteral(DecimalWithVariant {
                value: "42.4".to_string(),
                variant: None
            }))
        );
        assert_eq!(lexer.next().unwrap(), Ok(Token::Slash));
        assert_eq!(
            lexer.next().unwrap(),
            Ok(Token::IntegerLiteral("3".to_string()))
        );
    }

    #[test]
    fn equality() {
        let mut lexer = Token::lexer("a == b");
        assert_eq!(
            lexer.next().unwrap(),
            Ok(Token::Identifier("a".to_string()))
        );
        assert_eq!(lexer.next().unwrap(), Ok(Token::StructuralEqual));
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
        assert_eq!(lexer.next().unwrap(), Ok(Token::Is));
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
        assert_eq!(lexer.next().unwrap(), Ok(Token::Matches));
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
        assert_eq!(
            lexer.next().unwrap(),
            Ok(Token::IntegerLiteral("42".to_string()))
        );
        assert_eq!(lexer.next(), None);
    }

    #[test]
    fn divide() {
        let mut lexer = Token::lexer("8 /2");
        assert_eq!(
            lexer.next().unwrap(),
            Ok(Token::IntegerLiteral("8".to_string()))
        );
        assert_eq!(lexer.next().unwrap(), Ok(Token::Slash));
        assert_eq!(
            lexer.next().unwrap(),
            Ok(Token::IntegerLiteral("2".to_string()))
        );
        assert_eq!(lexer.next(), None);
    }
}
