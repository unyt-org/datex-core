use crate::ast::TypeExpression;
use crate::ast::chain::ApplyOperation;
use crate::decompiler::DecompileOptions;
use datex_core::ast::DatexExpression;
use datex_core::decompiler::Formatting;

#[derive(Clone, Default)]
enum BraceStyle {
    Curly,
    Square,
    Paren,
    #[default]
    None,
}

impl BraceStyle {
    fn open(&self) -> &str {
        match self {
            BraceStyle::Curly => "{",
            BraceStyle::Square => "[",
            BraceStyle::Paren => "(",
            BraceStyle::None => "",
        }
    }

    fn close(&self) -> &str {
        match self {
            BraceStyle::Curly => "}",
            BraceStyle::Square => "]",
            BraceStyle::Paren => ")",
            BraceStyle::None => "",
        }
    }
}

/// Converts a DatexExpression AST back into its source code representation as a String.
pub fn ast_to_source_code(
    ast: &DatexExpression,
    decompile_options: &DecompileOptions,
) -> String {
    match ast {
        DatexExpression::Integer(i) => i.to_string(),
        DatexExpression::TypedInteger(ti) => ti.to_string_with_suffix(),
        DatexExpression::Decimal(d) => d.to_string(),
        DatexExpression::TypedDecimal(td) => td.to_string_with_suffix(),
        DatexExpression::Boolean(b) => b.to_string(),
        DatexExpression::Text(t) => text_to_source_code(t),
        DatexExpression::Endpoint(e) => e.to_string(),
        DatexExpression::Null => "null".to_string(),
        DatexExpression::Identifier(l) => l.to_string(),
        DatexExpression::Map(map) => map_to_source_code(map, decompile_options),
        DatexExpression::List(elements) => {
            list_to_source_code(elements, decompile_options)
        }
        DatexExpression::CreateRef(expr) => {
            format!("&{}", ast_to_source_code(expr, decompile_options))
        }
        DatexExpression::CreateRefMut(expr) => {
            format!("&mut {}", ast_to_source_code(expr, decompile_options))
        }
        DatexExpression::CreateRefFinal(expr) => {
            format!("&final {}", ast_to_source_code(expr, decompile_options))
        }
        DatexExpression::BinaryOperation(operator, left, right, _type) => {
            let left_code = key_to_source_code(left, decompile_options);
            let right_code = key_to_source_code(right, decompile_options);
            let space = if matches!(
                decompile_options.formatting,
                Formatting::Compact
            ) {
                ""
            } else {
                " "
            };
            format!("{}{}{}{}{}", left_code, space, operator, space, right_code)
        }
        DatexExpression::ApplyChain(operand, applies) => {
            let mut applies_code = vec![];
            for apply in applies {
                match apply {
                    ApplyOperation::FunctionCall(args) => {
                        let args_code =
                            ast_to_source_code(args, decompile_options);
                        // apply()
                        if args_code.starts_with('(')
                            && args_code.ends_with(')')
                        {
                            applies_code.push(args_code);
                        }
                        // apply x
                        else {
                            applies_code.push(format!(" {}", args_code));
                        }
                    }
                    ApplyOperation::PropertyAccess(prop) => {
                        applies_code.push(format!(
                            ".{}",
                            key_to_source_code(prop, decompile_options)
                        ));
                    }
                    _ => todo!(),
                }
            }
            format!(
                "{}{}",
                ast_to_source_code(operand, decompile_options),
                applies_code.join("")
            )
        }

        DatexExpression::TypeExpression(type_expr) => {
            format!(
                "type({})",
                type_expression_to_source_code(type_expr, decompile_options)
            )
        }

        _ => todo!(),
    }
}

fn type_expression_to_source_code(
    type_expr: &TypeExpression,
    decompile_options: &DecompileOptions,
) -> String {
    match type_expr {
        TypeExpression::Integer(ti) => ti.to_string(),
        _ => todo!(),
    }
}

/// Converts a DatexExpression key into source code, adding parentheses if necessary
fn key_to_source_code(
    key: &DatexExpression,
    decompile_options: &DecompileOptions,
) -> String {
    match key {
        DatexExpression::Text(t) => key_to_string(t, decompile_options),
        DatexExpression::Integer(i) => i.to_string(),
        DatexExpression::TypedInteger(ti) => ti.to_string(),
        _ => format!("({})", ast_to_source_code(key, decompile_options)),
    }
}

/// Converts the contents of a DatexExpression::List into source code
fn list_to_source_code(
    list: &[DatexExpression],
    decompile_options: &DecompileOptions,
) -> String {
    let elements: Vec<String> = list
        .iter()
        .map(|e| ast_to_source_code(e, decompile_options))
        .collect();
    join_elements(elements, &decompile_options.formatting, BraceStyle::Square)
}

/// Converts the contents of a DatexExpression::Map into source code
fn map_to_source_code(
    map: &[(DatexExpression, DatexExpression)],
    decompile_options: &DecompileOptions,
) -> String {
    let elements: Vec<String> = map
        .iter()
        .map(|(k, v)| {
            format!(
                "{}:{}{}",
                key_to_source_code(k, decompile_options),
                if matches!(decompile_options.formatting, Formatting::Compact) {
                    ""
                } else {
                    " "
                },
                ast_to_source_code(v, decompile_options)
            )
        })
        .collect();
    join_elements(elements, &decompile_options.formatting, BraceStyle::Curly)
}

/// Converts a text string into a properly escaped source code representation
fn text_to_source_code(text: &str) -> String {
    // escape quotes and backslashes in text
    let text = text
        .replace('\\', r#"\\"#)
        .replace('"', r#"\""#)
        .replace('\u{0008}', r#"\b"#)
        .replace('\u{000c}', r#"\f"#)
        .replace('\r', r#"\r"#)
        .replace('\t', r#"\t"#)
        .replace('\u{000b}', r#"\v"#)
        .replace('\n', r#"\n"#);

    format!("\"{}\"", text)
}

/// Joins multiple elements into a single string with a comma separator, applying indentation and newlines for multiline formatting
/// E.g. "1", "2", "3" -> "1,\n 2,\n 3"
fn join_elements(
    elements: Vec<String>,
    formatting: &Formatting,
    brace_style: BraceStyle,
) -> String {
    match formatting {
        // no spaces or newlines for compact formatting
        Formatting::Compact => format!(
            "{}{}{}",
            brace_style.open(),
            elements.join(","),
            brace_style.close()
        ),
        // indent each element on a new line for multiline formatting, if the total length exceeds a threshold of 60 characters
        Formatting::Multiline { .. } => {
            let total_length: usize = elements.iter().map(|s| s.len()).sum();
            if total_length <= 60 {
                format!(
                    "{}{}{}",
                    brace_style.open(),
                    elements.join(", "),
                    brace_style.close()
                )
            } else {
                format!(
                    "{}\n{}\n{}",
                    brace_style.open(),
                    indent_lines(&elements.join(",\n"), *formatting),
                    brace_style.close()
                )
            }
        }
    }
}

/// Indents each line of the given string by the specified number of spaces if multiline formatting is used
fn indent_lines(s: &str, formatting: Formatting) -> String {
    match formatting {
        Formatting::Compact => s.to_string(),
        Formatting::Multiline { indent } => s
            .lines()
            .map(|line| format!("{}{}", " ".repeat(indent), line))
            .collect::<Vec<String>>()
            .join("\n"),
    }
}

fn is_alphanumeric_identifier(s: &str) -> bool {
    let mut chars = s.chars();

    // First character must be a-z, A-Z, or _
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }

    // Remaining characters: a-z, A-Z, 0-9, _, or -
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

fn key_to_string(key: &str, options: &DecompileOptions) -> String {
    // if text does not just contain a-z, A-Z, 0-9, _, and starts with a-z, A-Z,  _, add quotes
    if !options.json_compat && is_alphanumeric_identifier(key) {
        key.to_string()
    } else {
        text_to_source_code(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::values::core_values::decimal::Decimal;

    #[test]
    fn test_primitives() {
        let int_ast = DatexExpression::Integer(42.into());
        assert_eq!(
            ast_to_source_code(&int_ast, &DecompileOptions::default()),
            "42"
        );

        let typed_int_ast = DatexExpression::TypedInteger(42i8.into());
        assert_eq!(
            ast_to_source_code(&typed_int_ast, &DecompileOptions::default()),
            "42i8"
        );

        let decimal_ast = DatexExpression::Decimal(
            Decimal::from_string("1.23").unwrap().into(),
        );
        assert_eq!(
            ast_to_source_code(&decimal_ast, &DecompileOptions::default()),
            "1.23"
        );

        let decimal_ast = DatexExpression::Decimal(Decimal::Infinity.into());
        assert_eq!(
            ast_to_source_code(&decimal_ast, &DecompileOptions::default()),
            "infinity"
        );

        let decimal_ast = DatexExpression::Decimal(Decimal::NegInfinity.into());
        assert_eq!(
            ast_to_source_code(&decimal_ast, &DecompileOptions::default()),
            "-infinity"
        );

        let decimal_ast = DatexExpression::Decimal(Decimal::NaN.into());
        assert_eq!(
            ast_to_source_code(&decimal_ast, &DecompileOptions::default()),
            "nan"
        );

        let typed_decimal_ast = DatexExpression::TypedDecimal(2.71f32.into());
        assert_eq!(
            ast_to_source_code(
                &typed_decimal_ast,
                &DecompileOptions::default()
            ),
            "2.71f32"
        );

        let bool_ast = DatexExpression::Boolean(true);
        assert_eq!(
            ast_to_source_code(&bool_ast, &DecompileOptions::default()),
            "true"
        );

        let text_ast = DatexExpression::Text("Hello".to_string());
        assert_eq!(
            ast_to_source_code(&text_ast, &DecompileOptions::default()),
            "\"Hello\""
        );

        let null_ast = DatexExpression::Null;
        assert_eq!(
            ast_to_source_code(&null_ast, &DecompileOptions::default()),
            "null"
        );
    }

    #[test]
    fn test_list() {
        let list_ast = DatexExpression::List(vec![
            DatexExpression::Integer(1.into()),
            DatexExpression::Integer(2.into()),
            DatexExpression::Integer(3.into()),
        ]);
        assert_eq!(
            ast_to_source_code(&list_ast, &DecompileOptions::default()),
            "[1,2,3]"
        );

        let compile_options_multiline = DecompileOptions {
            formatting: Formatting::multiline(),
            ..Default::default()
        };

        // long list should be multi-line
        let long_list_ast = DatexExpression::List(vec![
            DatexExpression::Text("This is a long string".to_string()),
            DatexExpression::Text("Another long string".to_string()),
            DatexExpression::Text("Yet another long string".to_string()),
            DatexExpression::Text(
                "More long strings to increase length".to_string(),
            ),
            DatexExpression::Text("Final long string in the list".to_string()),
        ]);

        assert_eq!(
            ast_to_source_code(&long_list_ast, &compile_options_multiline),
            "[\n    \"This is a long string\",\n    \"Another long string\",\n    \"Yet another long string\",\n    \"More long strings to increase length\",\n    \"Final long string in the list\"\n]"
        );
    }

    #[test]
    fn test_map() {
        let map_ast = DatexExpression::Map(vec![
            (
                DatexExpression::Text("key1".to_string()),
                DatexExpression::Integer(1.into()),
            ),
            (
                DatexExpression::Text("key2".to_string()),
                DatexExpression::Text("two".to_string()),
            ),
            (
                DatexExpression::Integer(42.into()),
                DatexExpression::Boolean(true),
            ),
        ]);
        assert_eq!(
            ast_to_source_code(&map_ast, &DecompileOptions::default()),
            "{key1:1,key2:\"two\",42:true}"
        );
    }
}
