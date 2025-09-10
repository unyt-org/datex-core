use crate::ast::DatexExpression;
use crate::ast::DatexParserTrait;
use crate::ast::lexer::Token;
use chumsky::prelude::*;

pub fn text<'a>() -> impl DatexParserTrait<'a> {
    select! {
        Token::StringLiteral(s) => DatexExpression::Text(unescape_text(&s))
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
