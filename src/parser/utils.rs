use std::iter::Peekable;
use std::str::{Chars, FromStr};
use datex_core::values::core_values::integer::Integer;
use datex_core::values::core_values::integer::typed_integer::IntegerTypeVariant;
use crate::parser::lexer::{IntegerWithVariant, Token};
use crate::values::core_values::decimal::Decimal;
use crate::values::core_values::decimal::typed_decimal::{DecimalTypeVariant, TypedDecimal};
use crate::values::core_values::error::NumberParseError;
use crate::values::core_values::integer::typed_integer::TypedInteger;

pub enum IntegerOrDecimal {
    Integer(Integer),
    Decimal(Decimal),
    TypedInteger(TypedInteger),
    TypedDecimal(TypedDecimal),
}


pub enum IntegerOrTypedInteger {
    Integer(Integer),
    TypedInteger(TypedInteger),
}

/// Parses an integer literal with an integer part, an optional exponent part, and an optional variant suffix.
/// Returns either an Integer or a Decimal value.
pub fn parse_integer_literal(lit: String) -> Result<IntegerOrDecimal, NumberParseError> {
    // first consume all digits for the integer part, skipping underscores
    let mut chars = lit.chars().peekable();
    let integer_part = consume_digits_with_underscores(&mut chars);

    // check for exponent part
    let mut exponent_part = String::new();
    if let Some(&c) = chars.peek() {
        if c == 'e' || c == 'E' {
            chars.next();
            // optional + or -
            if let Some(&c2) = chars.peek() {
                if c2 == '-' {
                    exponent_part.push(c2);
                    chars.next();
                } else if c2 == '+' {
                    chars.next();
                }
            }
            // consume all digits for exponent part
            exponent_part += &consume_digits_with_underscores(&mut chars);
        }
    }

    // the rest is the variant suffix, if any
    let variant_part: String = chars.collect();

    // integer only if no exponent part
    if exponent_part.is_empty() {
        // no variant and no exponent -> plain integer
        if variant_part.is_empty() {
            Integer::from_string(&integer_part).map(IntegerOrDecimal::Integer)
        } 
        // variant -> distinguish between integer and decimal type variants
        else {
            // try to get integer type variant from variant part
            if let Ok(integer_variant) = IntegerTypeVariant::from_str(&variant_part) {
                TypedInteger::from_string_with_variant(&integer_part, integer_variant)
                    .map(IntegerOrDecimal::TypedInteger)
            }
            // otherwise try to parse as typed decimal
            else if let Ok(decimal_variant) = DecimalTypeVariant::from_str(&variant_part) {
                TypedDecimal::from_string_and_variant(&integer_part, decimal_variant)
                    .map(IntegerOrDecimal::TypedDecimal)
            }
            else {
                // should not happen if valid string literal is passed in
                unreachable!()
            }
        }
    }
    
    // decimal if exponent part is present
    else {
        let full_number = format!("{}e{}", integer_part, exponent_part);
        // no variant -> plain decimal with exponent
        if variant_part.is_empty() {
            Decimal::from_string(&full_number).map(IntegerOrDecimal::Decimal)
        }
        // decimal variant -> typed decimal with exponent
        else if let Ok(decimal_variant) = DecimalTypeVariant::from_str(&variant_part) {
            TypedDecimal::from_string_and_variant(&full_number, decimal_variant)
                .map(IntegerOrDecimal::TypedDecimal)
        }
        // otherwise invalid variant for decimal with exponent
        else {
            Err(NumberParseError::InvalidFormat)
        }
    }
}

fn consume_digits_with_underscores(chars: &mut Peekable<Chars>) -> String {
    let mut part = String::new();
    while let Some(&c) = chars.peek() {
        if c.is_digit(10) {
            part.push(c);
            chars.next();
        } else if c == '_' {
            // skip underscores
            chars.next();
        } else {
            break;
        }
    }
    part
}

pub fn parse_integer_with_variant(integer_with_variant: IntegerWithVariant, token: Token) -> Result<IntegerOrTypedInteger, NumberParseError> {
    let radix = match token {
        Token::BinaryIntegerLiteral(_) => 2,
        Token::OctalIntegerLiteral(_) => 8,
        Token::HexadecimalIntegerLiteral(_) => 16,
        _ => unreachable!(),
    };
    match integer_with_variant.variant {
        Some(var) => TypedInteger::from_string_radix_with_variant(&integer_with_variant.value[2..], radix, var)
            .map(IntegerOrTypedInteger::TypedInteger),
        None => Integer::from_string_radix(&integer_with_variant.value[2..], radix)
            .map(IntegerOrTypedInteger::Integer),
    }
}


/// Takes a literal text string input, e.g. ""Hello, world!"" or "'Hello, world!' or ""x\"""
/// and returns the unescaped text, e.g. "Hello, world!" or 'Hello, world!' or "x\""
pub fn unescape_text(text: &str) -> String {
    // remove first and last quote (double or single)
    let escaped = text[1..text.len() - 1]
        // Replace escape sequences with actual characters
        .replace(r#"\""#, "\"") // Replace \" with "
        .replace(r#"\'"#, "'") // Replace \' with '
        .replace(r#"\n"#, "\n") // Replace \n with newline
        .replace(r#"\r"#, "\r") // Replace \r with carriage return
        .replace(r#"\t"#, "\t") // Replace \t with tab
        .replace(r#"\b"#, "\x08") // Replace \b with backspace
        .replace(r#"\f"#, "\x0C") // Replace \f with form feed
        .replace(r#"\\"#, "\\") // Replace \\ with \
        // TODO #156 remove all other backslashes before any other character
        .to_string();
    // Decode unicode escapes, e.g. \u1234 or \uD800\uDC00
    decode_json_unicode_escapes(&escaped)
}

// TODO #352: double check if this works correctly for all edge cases
/// Decodes JSON-style unicode escape sequences, including surrogate pairs
fn decode_json_unicode_escapes(input: &str) -> String {
    let mut output = String::new();
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\\' && chars.peek() == Some(&'u') {
            chars.next(); // skip 'u'

            let mut code_unit = String::new();
            for _ in 0..4 {
                if let Some(c) = chars.next() {
                    code_unit.push(c);
                } else {
                    output.push_str("\\u");
                    output.push_str(&code_unit);
                    break;
                }
            }

            if let Ok(first_unit) = u16::from_str_radix(&code_unit, 16) {
                if (0xD800..=0xDBFF).contains(&first_unit) {
                    // High surrogate — look for low surrogate
                    if chars.next() == Some('\\') && chars.next() == Some('u') {
                        let mut low_code = String::new();
                        for _ in 0..4 {
                            if let Some(c) = chars.next() {
                                low_code.push(c);
                            } else {
                                output.push_str(&format!(
                                    "\\u{first_unit:04X}\\u{low_code}"
                                ));
                                break;
                            }
                        }

                        if let Ok(second_unit) =
                            u16::from_str_radix(&low_code, 16)
                            && (0xDC00..=0xDFFF).contains(&second_unit)
                        {
                            let combined = 0x10000
                                + (((first_unit - 0xD800) as u32) << 10)
                                + ((second_unit - 0xDC00) as u32);
                            if let Some(c) = char::from_u32(combined) {
                                output.push(c);
                                continue;
                            }
                        }

                        // Invalid surrogate fallback
                        output.push_str(&format!(
                            "\\u{first_unit:04X}\\u{low_code}"
                        ));
                    } else {
                        // Unpaired high surrogate
                        output.push_str(&format!("\\u{first_unit:04X}"));
                    }
                } else {
                    // Normal scalar value
                    if let Some(c) = char::from_u32(first_unit as u32) {
                        output.push(c);
                    } else {
                        output.push_str(&format!("\\u{first_unit:04X}"));
                    }
                }
            } else {
                output.push_str(&format!("\\u{code_unit}"));
            }
        } else {
            output.push(ch);
        }
    }

    output
}
