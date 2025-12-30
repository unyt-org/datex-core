use crate::values::core_values::decimal::Decimal;
use crate::ast::lexer::Token;
use crate::ast::spanned::Spanned;
use crate::ast::structs::expression::{DatexExpression, DatexExpressionData};
use crate::parser::{SpannedParserError, Parser};
use crate::parser::errors::ParserError;

impl Parser {
    pub(crate) fn parse_atom(&mut self) -> Result<DatexExpression, SpannedParserError> {
        Ok(match self.peek()?.token.clone() {

            Token::LeftCurly => self.parse_map()?,
            Token::LeftBracket => self.parse_list()?,
            Token::LeftParen => self.parse_statements()?,

            Token::True => {
                DatexExpressionData::Boolean(true)
                    .with_span(self.advance()?.span)
            }
            Token::False => {
                DatexExpressionData::Boolean(false)
                    .with_span(self.advance()?.span)
            }
            Token::Null => {
                DatexExpressionData::Null
                    .with_span(self.advance()?.span)
            }
            Token::Identifier(name) => {
                DatexExpressionData::Identifier(name)
                    .with_span(self.advance()?.span)
            }
            Token::StringLiteral(value) => {
                DatexExpressionData::Text(unescape_text(&value))
                    .with_span(self.advance()?.span)
            }
            Token::Infinity => {
                DatexExpressionData::Decimal(Decimal::Infinity)
                    .with_span(self.advance()?.span)
            }
            Token::Nan => {
                DatexExpressionData::Decimal(Decimal::Nan)
                    .with_span(self.advance()?.span)
            }

            _ => return Err(SpannedParserError {
                error: ParserError::UnexpectedToken,
                span: self.peek()?.span.clone()
            })
        })
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




#[cfg(test)]
mod tests {
    use crate::ast::structs::expression::DatexExpressionData;
    use crate::parser::tests::parse;
    use crate::values::core_values::decimal::Decimal;

    #[test]
    fn parse_boolean_true() {
        let expr = parse("true");
        assert_eq!(expr.data, DatexExpressionData::Boolean(true));
    }

    #[test]
    fn parse_boolean_false() {
        let expr = parse("false");
        assert_eq!(expr.data, DatexExpressionData::Boolean(false));
    }

    #[test]
    fn parse_null() {
        let expr = parse("null");
        assert_eq!(expr.data, DatexExpressionData::Null);
    }

    #[test]
    fn parse_identifier() {
        let expr = parse("myVar");
        assert_eq!(expr.data, DatexExpressionData::Identifier("myVar".to_string()));
    }

    #[test]
    fn parse_string_literal() {
        let expr = parse("\"Hello, World!\"");
        assert_eq!(expr.data, DatexExpressionData::Text("Hello, World!".to_string()));

        let expr2 = parse("'Single quotes'");
        assert_eq!(expr2.data, DatexExpressionData::Text("Single quotes".to_string()));
    }

    #[test]
    fn parse_infinity() {
        let expr = parse("Infinity");
        assert_eq!(expr.data, DatexExpressionData::Decimal(Decimal::Infinity));
    }

    #[test]
    fn parse_nan() {
        let expr = parse("NaN");
        assert_eq!(expr.data, DatexExpressionData::Decimal(Decimal::Nan));
    }
}