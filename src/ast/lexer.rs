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

    #[token(".")]
    Dot,
    // pointer address (e.g. $1234ab, exactly 3, 5 or 26 bytes)
    #[regex(r"\$(?:[0-9a-fA-F]{6}|[0-9a-fA-F]{10}|[0-9a-fA-F]{52})", allocated_string)] PointerAddress(String),

    // decimal literals (infinity, nan)
    #[regex(r"[Ii]nfinity")] Infinity,
    #[regex(r"(?:nan|NaN)")] Nan,

    // decimal part / integer
    // for handling decimals and integers in decimal
    // a decimal will consume 3 parts:
    //  - left of dot (DecimalNumericLiteral)
    //  - dot (Dot)
    //  - right of dot (DecimalNumericLiteral)
    #[regex(r"\d+(?:_\d+)*(?:[eE][+-]?\d+(?:_\d+)*)?(?:f32|f64|u8|u16|u32|u64|u128|i8|i16|i32|i64|i128|big)?", numeric_parts)]
    DecimalNumericLiteral(NumericLiteralParts),

    // Binary / Octal / Hex integers with optional suffix
    #[regex(
        r"0[bB][01](?:_?[01])*(?:u8|u16|u32|u64|u128|i8|i16|i32|i64|i128|big)?",
        parse_typed_literal::<IntegerTypeVariant>
    )]
    BinaryIntegerLiteral(IntegerLiteral),

    #[regex(
        r"0[oO][0-7](?:_?[0-7])*(?:u8|u16|u32|u64|u128|i8|i16|i32|i64|i128|big)?",
        parse_typed_literal::<IntegerTypeVariant>
    )]
    OctalIntegerLiteral(IntegerLiteral),

    #[regex(
        r"0[xX][0-9a-fA-F](?:_?[0-9a-fA-F])*(?:u8|u16|u32|u64|u128|i8|i16|i32|i64|i128|big)?",
        parse_typed_literal::<IntegerTypeVariant>
    )]
    HexadecimalIntegerLiteral(IntegerLiteral),


    // fraction (e.g. 1/2)
    #[regex(r"FIXMEREMOVWE\d+/\d+", allocated_string)] 
    FractionLiteral(String),

    #[regex(r#"[a-z0-9]*("(?:\\.|[^\\"])*"|'(?:\\.|[^\\'])*')"#, allocated_string)] StringLiteral(String),


    #[regex(r"@[+@]?[a-zA-Z0-9_-]+", allocated_string)] Endpoint(String),

    // identifiers
    #[regex(r"[_\p{L}][_\p{L}\p{N}]*", allocated_string, priority=1)] Identifier(String),

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
            Token::Infinity => Some("infinity"),
            Token::Nan => Some("nan"),
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
            Token::DecimalNumericLiteral(_) => "decimal integer literal",
            Token::BinaryIntegerLiteral(_) => "binary integer literal",
            Token::OctalIntegerLiteral(_) => "octal integer literal",
            Token::HexadecimalIntegerLiteral(_) => {
                "hexadecimal integer literal"
            }
            Token::StringLiteral(_) => "string literal",
            Token::Endpoint(_) => "endpoint",
            Token::Slot(_) => "slot",
            Token::NamedSlot(_) => "named slot",
            Token::Error => "error",
            Token::Identifier(s) => s,
            Token::Matches => "matches",
            Token::If => "if",
            Token::Else => "else",
            e => core::todo!("#367 Unhandled token in as_string: {:?}", e),
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
            Ok(Token::DecimalNumericLiteral("42".into()))
        );
    }

    #[test]
    fn integer_type() {
        let mut lexer = Token::lexer("42u8");
        let res = lexer.next().unwrap();
        if let Ok(Token::DecimalNumericLiteral(literal)) = res {
            assert_eq!(literal.integer_part, "42");
            assert_eq!(
                literal.variant_part,
                Some(IntegerTypeVariant::U8.to_string())
            );
            assert_eq!(format!("{}", literal), "42u8".to_string());
        } else {
            core::panic!("Expected DecimalIntegerLiteral with variant U8");
        }

        let mut lexer = Token::lexer("42");
        let res = lexer.next().unwrap();
        if let Ok(Token::DecimalNumericLiteral(literal)) = res {
            assert_eq!(literal, "42".into());
        } else {
            core::panic!("Expected DecimalIntegerLiteral with no variant");
        }
    }

    #[test]
    fn integer_with_type() {
        let mut lexer = Token::lexer("42u8");
        assert_eq!(
            lexer.next().unwrap(),
            Ok(Token::DecimalNumericLiteral(NumericLiteralParts {
                exponent_part: None,
                integer_part: "42".to_string(),
                variant_part: Some("u8".into())
            }))
        );

        let mut lexer = Token::lexer("42i32");
        assert_eq!(
            lexer.next().unwrap(),
            Ok(Token::DecimalNumericLiteral(NumericLiteralParts {
                exponent_part: None,
                integer_part: "42".to_string(),
                variant_part: Some("i32".into())
            }))
        );

        let mut lexer = Token::lexer("42big");
        assert_eq!(
            lexer.next().unwrap(),
            Ok(Token::DecimalNumericLiteral(NumericLiteralParts {
                exponent_part: None,
                integer_part: "42".to_string(),
                variant_part: Some("big".into())
            }))
        );
    }

    #[test]
    fn decimal() {
        let mut lexer = Token::lexer("3.14");
        assert_eq!(
            lexer.collect::<Vec<_>>(),
            vec![
                Ok(Token::DecimalNumericLiteral(NumericLiteralParts {
                    integer_part: "3".to_string(),
                    exponent_part: None,
                    variant_part: None,
                })),
                Ok(Token::Dot),
                Ok(Token::DecimalNumericLiteral(NumericLiteralParts {
                    integer_part: "14".to_string(),
                    exponent_part: None,
                    variant_part: None,
                }))
            ]
        );

        // leading dot without leading zero must be tokenized as Dot + DecimalIntegerLiteral
        // to avoid ambiguity with property access like `myStruct.5.test`
        let mut lexer = Token::lexer(".5");
        assert_eq!(lexer.next().unwrap(), Ok(Token::Dot));
        assert_eq!(
            lexer.next().unwrap(),
            Ok(Token::DecimalNumericLiteral(NumericLiteralParts {
                integer_part: "5".to_string(),
                exponent_part: None,
                variant_part: None,
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

    // #[test]
    // fn fraction() {
    //     let mut lexer = Token::lexer("1/2");
    //     assert_eq!(
    //         lexer.next().unwrap(),
    //         Ok(Token::FractionLiteral("1/2".to_string()))
    //     );

    //     let mut lexer = Token::lexer("3/4");
    //     assert_eq!(
    //         lexer.next().unwrap(),
    //         Ok(Token::FractionLiteral("3/4".to_string()))
    //     );

    //     let mut lexer = Token::lexer("5111/6");
    //     assert_eq!(
    //         lexer.next().unwrap(),
    //         Ok(Token::FractionLiteral("5111/6".to_string()))
    //     );
    // }

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
            Ok(Token::DecimalNumericLiteral("1_000".into()))
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
    fn decimals() {
        // exponent, variant
        let lexer = Token::lexer("10.234_567e-8f32");
        assert_eq!(
            lexer.collect::<Vec<_>>(),
            vec![
                Ok(Token::DecimalNumericLiteral(NumericLiteralParts {
                    integer_part: "10".to_string(),
                    exponent_part: None,
                    variant_part: None
                })),
                Ok(Token::Dot),
                Ok(Token::DecimalNumericLiteral(NumericLiteralParts {
                    integer_part: "234567".to_string(),
                    exponent_part: Some("-8".to_string()),
                    variant_part: Some("f32".to_string())
                }))
            ]
        );

        // exponent, no variant
        let lexer = Token::lexer("10.234_567e-8");
        assert_eq!(
            lexer.collect::<Vec<_>>(),
            vec![
                Ok(Token::DecimalNumericLiteral(NumericLiteralParts {
                    integer_part: "10".to_string(),
                    exponent_part: None,
                    variant_part: None
                })),
                Ok(Token::Dot),
                Ok(Token::DecimalNumericLiteral(NumericLiteralParts {
                    integer_part: "234567".to_string(),
                    exponent_part: Some("-8".to_string()),
                    variant_part: None
                }))
            ]
        );

        // no exponent, no variant
        let lexer = Token::lexer("0.123_456");
        assert_eq!(
            lexer.collect::<Vec<_>>(),
            vec![
                Ok(Token::DecimalNumericLiteral(NumericLiteralParts {
                    integer_part: "0".to_string(),
                    exponent_part: None,
                    variant_part: None
                })),
                Ok(Token::Dot),
                Ok(Token::DecimalNumericLiteral(NumericLiteralParts {
                    integer_part: "123456".to_string(),
                    exponent_part: None,
                    variant_part: None
                }))
            ]
        );

        // no exponent, variant
        let lexer = Token::lexer("1_000.123_456f32");
        assert_eq!(
            lexer.collect::<Vec<_>>(),
            vec![
                Ok(Token::DecimalNumericLiteral(NumericLiteralParts {
                    integer_part: "1000".to_string(),
                    exponent_part: None,
                    variant_part: None
                })),
                Ok(Token::Dot),
                Ok(Token::DecimalNumericLiteral(NumericLiteralParts {
                    integer_part: "123456".to_string(),
                    exponent_part: None,
                    variant_part: Some("f32".to_string())
                }))
            ]
        );
    }

    #[test]
    fn add() {
        let mut lexer = Token::lexer("1 + 2");
        assert_eq!(
            lexer.next().unwrap(),
            Ok(Token::DecimalNumericLiteral("1".into()))
        );
        assert_eq!(lexer.next().unwrap(), Ok(Token::Whitespace));
        assert_eq!(lexer.next().unwrap(), Ok(Token::Plus));
        assert_eq!(lexer.next().unwrap(), Ok(Token::Whitespace));
        assert_eq!(
            lexer.next().unwrap(),
            Ok(Token::DecimalNumericLiteral("2".into()))
        );
        assert_eq!(lexer.next(), None);
    }

    #[test]
    fn invalid_fraction() {
        let mut lexer = Token::lexer("42.4/3");
        assert_eq!(
            lexer.next().unwrap(),
            Ok(Token::DecimalNumericLiteral("42".into()))
        );
        assert_eq!(lexer.next().unwrap(), Ok(Token::Dot));
        assert_eq!(
            lexer.next().unwrap(),
            Ok(Token::DecimalNumericLiteral("4".into()))
        );
        assert_eq!(lexer.next().unwrap(), Ok(Token::Slash));
        assert_eq!(
            lexer.next().unwrap(),
            Ok(Token::DecimalNumericLiteral("3".into()))
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
            Ok(Token::DecimalNumericLiteral("42".into()))
        );
        assert_eq!(lexer.next(), None);
    }

    #[test]
    fn divide() {
        let mut lexer = Token::lexer("8 /2");
        assert_eq!(
            lexer.next().unwrap(),
            Ok(Token::DecimalNumericLiteral("8".into()))
        );
        assert_eq!(lexer.next().unwrap(), Ok(Token::Whitespace));
        assert_eq!(lexer.next().unwrap(), Ok(Token::Slash));
        assert_eq!(
            lexer.next().unwrap(),
            Ok(Token::DecimalNumericLiteral("2".into()))
        );
        assert_eq!(lexer.next(), None);
    }
}
