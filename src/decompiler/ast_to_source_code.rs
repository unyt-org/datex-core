use crate::ast::chain::ApplyOperation;
use crate::ast::literal;
use crate::ast::tree::{
    DatexExpression, DatexExpressionData, TypeExpression, VariableAccess,
    VariableAssignment, VariableDeclaration,
};
use crate::decompiler::DecompileOptions;
use crate::decompiler::formatter::Formatter;
use crate::f_fmt;
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
    return "ZES".to_string();
    /*
    let formatter = Formatter::new(decompile_options.formatting.clone());

    match &ast.data {
        DatexExpressionData::Integer(i) => i.to_string(),
        DatexExpressionData::TypedInteger(ti) => ti.to_string_with_suffix(),
        DatexExpressionData::Decimal(d) => d.to_string(),
        DatexExpressionData::TypedDecimal(td) => td.to_string_with_suffix(),
        DatexExpressionData::Boolean(b) => b.to_string(),
        DatexExpressionData::Text(t) => text_to_source_code(t),
        DatexExpressionData::Endpoint(e) => e.to_string(),
        DatexExpressionData::Null => "null".to_string(),
        DatexExpressionData::Identifier(l) => l.to_string(),
        DatexExpressionData::Map(map) => {
            map_to_source_code(map, decompile_options)
        }
        DatexExpressionData::List(elements) => {
            list_to_source_code(elements, decompile_options)
        }
        DatexExpressionData::CreateRef(expr) => {
            format!("&{}", ast_to_source_code(expr, decompile_options))
        }
        DatexExpressionData::CreateRefMut(expr) => {
            format!("&mut {}", ast_to_source_code(expr, decompile_options))
        }
        DatexExpressionData::CreateRefFinal(expr) => {
            format!("&final {}", ast_to_source_code(expr, decompile_options))
        }
        DatexExpressionData::BinaryOperation(operator, left, right, _type) => {
            let left_code = key_to_source_code(left, decompile_options);
            let right_code = key_to_source_code(right, decompile_options);
            f_fmt!(formatter, "{}%s{}%s{}", left_code, operator, right_code)
        }
        DatexExpressionData::ApplyChain(operand, applies) => {
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
                    _ => todo!("#419 Undescribed by author."),
                }
            }
            format!(
                "{}{}",
                ast_to_source_code(operand, decompile_options),
                applies_code.join("")
            )
        }
        DatexExpressionData::TypeExpression(type_expr) => {
            format!(
                "type({})",
                type_expression_to_source_code(type_expr, decompile_options)
            )
        }
        DatexExpressionData::Recover => unreachable!(
            "DatexExpressionData::Recover should not appear in a valid AST"
        ),
        DatexExpressionData::Statements(statements) => {
            let statements_code: Vec<String> = statements
                .statements
                .iter()
                .map(|stmt| {
                    let code = ast_to_source_code(stmt, decompile_options);
                    f_fmt!(formatter, "{};%n", code)
                })
                .collect();
            statements_code.join("")
        }
        DatexExpressionData::GetReference(pointer_address) => {
            format!("{}", pointer_address) // FIXME
        }
        DatexExpressionData::Conditional {
            condition,
            then_branch,
            else_branch,
        } => todo!(),
        DatexExpressionData::VariableDeclaration(VariableDeclaration {
            id: _,
            kind,
            name,
            init_expression,
            type_annotation,
        }) => {
            let mut code = String::new();
            code.push_str(&kind.to_string());
            code.push_str(name);
            if let Some(type_annotation) = type_annotation {
                code.push_str(&f_fmt!(formatter, ":%s"));
                code.push_str(&type_expression_to_source_code(
                    type_annotation,
                    decompile_options,
                ));
            }
            code.push_str(&formatter.optional_pad("="));
            code.push_str(&ast_to_source_code(
                init_expression,
                decompile_options,
            ));
            code
        }
        DatexExpressionData::VariableAssignment(VariableAssignment {
            id: _,
            expression,
            name,
            operator,
        }) => {
            let mut code = String::new();
            code.push_str(name);
            code.push_str(&formatter.optional_pad(&operator.to_string()));
            code.push_str(&ast_to_source_code(expression, decompile_options));
            code
        }
        DatexExpressionData::VariableAccess(VariableAccess {
            name, ..
        }) => name.to_string(),
        DatexExpressionData::TypeDeclaration {
            id: _,
            name,
            value,
            hoisted: _,
        } => {
            f_fmt!(
                formatter,
                "type {}%s=%s{}",
                name,
                type_expression_to_source_code(value, decompile_options)
            )
        }
        DatexExpressionData::Type(type_expression) => {
            type_expression_to_source_code(type_expression, decompile_options)
        }
        DatexExpressionData::FunctionDeclaration {
            name,
            parameters,
            return_type,
            body,
        } => {
            let params_code: Vec<String> = parameters
                .iter()
                .map(|(param_name, param_type)| {
                    f_fmt!(
                        formatter,
                        "{}:%s{}",
                        param_name,
                        type_expression_to_source_code(
                            param_type,
                            decompile_options
                        )
                    )
                })
                .collect();
            let return_type_code = if let Some(return_type) = return_type {
                format!(
                    "{}{}",
                    formatter.optional_pad("->"),
                    type_expression_to_source_code(
                        return_type,
                        decompile_options
                    )
                )
            } else {
                "".to_string()
            };
            let body_code = ast_to_source_code(body, decompile_options);
            f_fmt!(
                formatter,
                "fn {}({}){}{}%s(%n{}%n)",
                name,
                params_code.join(", "),
                return_type_code,
                body_code
            )
        }
        DatexExpressionData::Deref(datex_expression) => {
            format!(
                "*{}",
                ast_to_source_code(datex_expression, decompile_options)
            )
        }
        DatexExpressionData::Slot(slot) => slot.to_string(),
        DatexExpressionData::SlotAssignment(slot, datex_expression) => {
            format!(
                "{}{}{}",
                slot,
                formatter.optional_pad("="),
                ast_to_source_code(datex_expression, decompile_options)
            )
        }
        DatexExpressionData::PointerAddress(pointer_address) => {
            pointer_address.to_string()
        }
        DatexExpressionData::ComparisonOperation(
            comparison_operator,
            datex_expression,
            datex_expression1,
        ) => {
            f_fmt!(
                formatter,
                "{}%s{}%s{}",
                ast_to_source_code(datex_expression, decompile_options),
                comparison_operator,
                ast_to_source_code(datex_expression1, decompile_options)
            )
        }
        DatexExpressionData::DerefAssignment {
            operator,
            deref_count,
            deref_expression,
            assigned_expression,
        } => {
            let deref_prefix = "*".repeat(*deref_count);
            format!(
                "{}{}{}{}",
                deref_prefix,
                ast_to_source_code(deref_expression, decompile_options),
                formatter.optional_pad(&operator.to_string()),
                ast_to_source_code(assigned_expression, decompile_options)
            )
        }
        DatexExpressionData::UnaryOperation(unary_operation) => {
            format!(
                "{}{}",
                unary_operation.operator,
                ast_to_source_code(
                    &unary_operation.expression,
                    decompile_options
                )
            )
        }
        DatexExpressionData::Placeholder => "?".to_string(),
        DatexExpressionData::RemoteExecution(
            datex_expression,
            datex_expression1,
        ) => {
            format!(
                "{}{}{}",
                ast_to_source_code(datex_expression, decompile_options),
                formatter.optional_pad("::"),
                ast_to_source_code(datex_expression1, decompile_options)
            )
        }
    } */
}

fn type_expression_to_source_code(
    type_expr: &TypeExpression,
    decompile_options: &DecompileOptions,
) -> String {
    match type_expr {
        TypeExpression::Integer(ti) => ti.to_string(),
        TypeExpression::Decimal(td) => td.to_string(),
        TypeExpression::Boolean(boolean) => boolean.to_string(),
        TypeExpression::Text(text) => text.to_string(),
        TypeExpression::Endpoint(endpoint) => endpoint.to_string(),
        TypeExpression::Null => "null".to_string(),
        TypeExpression::Ref(inner) => {
            format!(
                "&{}",
                type_expression_to_source_code(inner, decompile_options)
            )
        }
        TypeExpression::RefMut(inner) => {
            format!(
                "&mut {}",
                type_expression_to_source_code(inner, decompile_options)
            )
        }
        TypeExpression::RefFinal(inner) => {
            format!(
                "&final {}",
                type_expression_to_source_code(inner, decompile_options)
            )
        }
        TypeExpression::Literal(literal) => literal.to_string(),
        TypeExpression::Variable(_, name) => name.to_string(),
        TypeExpression::GetReference(pointer_address) => {
            format!("{}", pointer_address) // FIXME
        }
        TypeExpression::TypedInteger(typed_integer) => {
            typed_integer.to_string_with_suffix()
        }
        TypeExpression::TypedDecimal(typed_decimal) => {
            typed_decimal.to_string_with_suffix()
        }
        TypeExpression::StructuralList(type_expressions) => {
            let elements: Vec<String> = type_expressions
                .iter()
                .map(|e| type_expression_to_source_code(e, decompile_options))
                .collect();
            format!("[{}]", elements.join(", "))
        }
        TypeExpression::FixedSizeList(type_expression, _) => todo!(),
        TypeExpression::SliceList(type_expression) => todo!(),
        TypeExpression::Intersection(type_expressions) => todo!(),
        TypeExpression::Union(type_expressions) => todo!(),
        TypeExpression::Generic(_, type_expressions) => todo!(),
        TypeExpression::Function {
            parameters,
            return_type,
        } => todo!(),
        TypeExpression::StructuralMap(items) => todo!(),
    }
}

/// Converts a DatexExpression key into source code, adding parentheses if necessary
fn key_to_source_code(
    key: &DatexExpression,
    decompile_options: &DecompileOptions,
) -> String {
    match &key.data {
        DatexExpressionData::Text(t) => key_to_string(t, decompile_options),
        DatexExpressionData::Integer(i) => i.to_string(),
        DatexExpressionData::TypedInteger(ti) => ti.to_string(),
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
    // TODO #422: Move this to text (as unescape_text is required in the Display)
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
    return "ZES".to_string();
    /*
    let formatter = Formatter::new(formatting.clone());
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
                    formatter.indent_lines(&elements.join(",\n")),
                    brace_style.close()
                )
            }
        }
    } */
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
    use std::str::FromStr;

    use super::*;
    use crate::{
        ast::{assignment_operation::AssignmentOperator, tree::VariableKind},
        libs::core::{CoreLibPointerId, get_core_lib_type},
        values::core_values::decimal::Decimal,
    };

    #[test]
    fn test_primitives() {
        let int_ast = DatexExpressionData::Integer(42.into());
        assert_eq!(
            ast_to_source_code(
                &int_ast.with_default_span(),
                &DecompileOptions::default()
            ),
            "42"
        );

        let typed_int_ast = DatexExpressionData::TypedInteger(42i8.into());
        assert_eq!(
            ast_to_source_code(
                &typed_int_ast.with_default_span(),
                &DecompileOptions::default()
            ),
            "42i8"
        );

        let decimal_ast =
            DatexExpressionData::Decimal(Decimal::from_string("1.23").unwrap());
        assert_eq!(
            ast_to_source_code(
                &decimal_ast.with_default_span(),
                &DecompileOptions::default()
            ),
            "1.23"
        );

        let decimal_ast = DatexExpressionData::Decimal(Decimal::Infinity);
        assert_eq!(
            ast_to_source_code(
                &decimal_ast.with_default_span(),
                &DecompileOptions::default()
            ),
            "infinity"
        );

        let decimal_ast = DatexExpressionData::Decimal(Decimal::NegInfinity);
        assert_eq!(
            ast_to_source_code(
                &decimal_ast.with_default_span(),
                &DecompileOptions::default()
            ),
            "-infinity"
        );

        let decimal_ast = DatexExpressionData::Decimal(Decimal::NaN);
        assert_eq!(
            ast_to_source_code(
                &decimal_ast.with_default_span(),
                &DecompileOptions::default()
            ),
            "nan"
        );

        let typed_decimal_ast =
            DatexExpressionData::TypedDecimal(2.71f32.into());
        assert_eq!(
            ast_to_source_code(
                &typed_decimal_ast.with_default_span(),
                &DecompileOptions::default()
            ),
            "2.71f32"
        );

        let bool_ast = DatexExpressionData::Boolean(true);
        assert_eq!(
            ast_to_source_code(
                &bool_ast.with_default_span(),
                &DecompileOptions::default()
            ),
            "true"
        );

        let text_ast = DatexExpressionData::Text("Hello".to_string());
        assert_eq!(
            ast_to_source_code(
                &text_ast.with_default_span(),
                &DecompileOptions::default()
            ),
            "\"Hello\""
        );

        let null_ast = DatexExpressionData::Null;
        assert_eq!(
            ast_to_source_code(
                &null_ast.with_default_span(),
                &DecompileOptions::default()
            ),
            "null"
        );
    }

    #[test]
    fn test_list() {
        let list_ast = DatexExpressionData::List(vec![
            DatexExpressionData::Integer(1.into()).with_default_span(),
            DatexExpressionData::Integer(2.into()).with_default_span(),
            DatexExpressionData::Integer(3.into()).with_default_span(),
        ]);
        assert_eq!(
            ast_to_source_code(
                &list_ast.with_default_span(),
                &DecompileOptions::default()
            ),
            "[1,2,3]"
        );

        let compile_options_multiline = DecompileOptions {
            formatting: Formatting::multiline(),
            ..Default::default()
        };

        // long list should be multi-line
        let long_list_ast = DatexExpressionData::List(vec![
            DatexExpressionData::Text("This is a long string".to_string())
                .with_default_span(),
            DatexExpressionData::Text("Another long string".to_string())
                .with_default_span(),
            DatexExpressionData::Text("Yet another long string".to_string())
                .with_default_span(),
            DatexExpressionData::Text(
                "More long strings to increase length".to_string(),
            )
            .with_default_span(),
            DatexExpressionData::Text(
                "Final long string in the list".to_string(),
            )
            .with_default_span(),
        ]);

        assert_eq!(
            ast_to_source_code(
                &long_list_ast.with_default_span(),
                &compile_options_multiline
            ),
            "[\n    \"This is a long string\",\n    \"Another long string\",\n    \"Yet another long string\",\n    \"More long strings to increase length\",\n    \"Final long string in the list\"\n]"
        );
    }

    #[test]
    fn test_map() {
        let map_ast = DatexExpressionData::Map(vec![
            (
                DatexExpressionData::Text("key1".to_string())
                    .with_default_span(),
                DatexExpressionData::Integer(1.into()).with_default_span(),
            ),
            (
                DatexExpressionData::Text("key2".to_string())
                    .with_default_span(),
                DatexExpressionData::Text("two".to_string())
                    .with_default_span(),
            ),
            (
                DatexExpressionData::Integer(42.into()).with_default_span(),
                DatexExpressionData::Boolean(true).with_default_span(),
            ),
        ]);
        assert_eq!(
            ast_to_source_code(
                &map_ast.with_default_span(),
                &DecompileOptions::default()
            ),
            "{key1:1,key2:\"two\",42:true}"
        );
    }

    #[test]
    fn test_deref() {
        let deref_ast = DatexExpressionData::Deref(Box::new(
            DatexExpressionData::VariableAccess(VariableAccess {
                id: 0,
                name: "ptr".to_string(),
            })
            .with_default_span(),
        ));
        assert_eq!(
            ast_to_source_code(
                &deref_ast.with_default_span(),
                &DecompileOptions::default()
            ),
            "*ptr"
        );
    }

    #[test]
    fn test_deref_assignment() {
        let deref_assign_ast = DatexExpressionData::DerefAssignment {
            operator: AssignmentOperator::Assign,
            deref_count: 2,
            deref_expression: Box::new(
                DatexExpressionData::VariableAccess(VariableAccess {
                    id: 0,
                    name: "ptr".to_string(),
                })
                .with_default_span(),
            ),
            assigned_expression: Box::new(
                DatexExpressionData::Integer(42.into()).with_default_span(),
            ),
        };
        assert_eq!(
            ast_to_source_code(
                &deref_assign_ast.with_default_span(),
                &DecompileOptions::default()
            ),
            "**ptr=42"
        );
    }

    #[test]
    fn test_variable_declaration() {
        let var_decl_ast =
            DatexExpressionData::VariableDeclaration(VariableDeclaration {
                id: Some(0),
                kind: VariableKind::Const,
                name: "x".to_string(),
                init_expression: Box::new(
                    DatexExpressionData::Integer(10.into()).with_default_span(),
                ),
                type_annotation: Some(TypeExpression::RefMut(Box::new(
                    TypeExpression::Literal("integer/u8".to_owned()),
                ))),
            });
        assert_eq!(
            ast_to_source_code(
                &var_decl_ast.with_default_span(),
                &DecompileOptions::default()
            ),
            "const x:integer/u8=10"
        );
    }
}
