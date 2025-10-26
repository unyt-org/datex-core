use std::fmt::{self};

use crate::ast::tree::{
    ApplyChain, BinaryOperation, ComparisonOperation, Conditional,
    DerefAssignment, List, Map, RemoteExecution, SlotAssignment,
    TypeDeclaration,
};
use crate::{
    ast::{
        chain::ApplyOperation,
        tree::{
            DatexExpression, DatexExpressionData, FunctionDeclaration,
            TypeExpressionData, VariableAccess, VariableAssignment,
            VariableDeclaration,
        },
    },
    decompiler::FormattingMode,
};

#[derive(Clone, Default)]
pub enum BraceStyle {
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

/// Check if the given string is a valid alphanumeric identifier (a-z, A-Z, 0-9, _ , -), starting with a-z, A-Z, or _
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

pub struct AstToSourceCodeFormatter {
    mode: FormattingMode,
    json_compat: bool,
    colorized: bool,
    indent_level: usize,
    indent_size: usize,
    use_spaces: bool,
    add_variant_suffix: bool,
}

#[macro_export]
macro_rules! ast_fmt {
    ($fmtter:expr, $fmt:expr $(, $args:expr )* $(,)?) => {
        $fmtter.fmt(std::format_args!($fmt $(, $args )*))
    };
}
impl AstToSourceCodeFormatter {
    const MAX_INLINE: usize = 60;

    pub fn new(
        mode: FormattingMode,
        json_compat: bool,
        colorized: bool,
    ) -> Self {
        let add_variant_suffix = match mode {
            FormattingMode::Compact => false,
            FormattingMode::Pretty => !json_compat,
        };
        Self {
            mode,
            json_compat,
            colorized,
            indent_level: 0,
            indent_size: 2,
            add_variant_suffix,
            use_spaces: true,
        }
    }

    /// Whether to add type variant suffixes to typed integers and decimals
    fn add_variant_suffix(&self) -> bool {
        if self.json_compat {
            false
        } else {
            self.add_variant_suffix
        }
    }

    /// Return the character used for indentation
    fn indent_char(&self) -> &'static char {
        if self.use_spaces { &' ' } else { &'\t' }
    }

    /// Return the indentation as a string
    fn indent(&self) -> String {
        self.indent_char().to_string().repeat(self.indent_size)
    }

    /// Return a space or empty string based on formatting mode
    fn space(&self) -> &'static str {
        if matches!(self.mode, FormattingMode::Compact) {
            ""
        } else {
            " "
        }
    }

    // Return a newline or empty string based on formatting mode
    fn newline(&self) -> &'static str {
        if matches!(self.mode, FormattingMode::Compact) {
            ""
        } else {
            "\n"
        }
    }

    /// Write formatted output with indentation and optional %s / %n expansion
    pub fn fmt(&self, args: fmt::Arguments) -> String {
        let mut intermediate = String::new();
        fmt::write(&mut intermediate, args).expect("formatting failed");
        intermediate
            .replace("%n", self.newline())
            .replace("%s", self.space())
    }

    /// Pad the given string with spaces if not in compact mode
    fn pad(&self, s: &str) -> String {
        if matches!(self.mode, FormattingMode::Compact) {
            s.to_string()
        } else {
            format!("{}{}{}", self.space(), s, self.space())
        }
    }

    /// Escape text to be a valid source code string literal
    fn text_to_source_code(&self, text: &str) -> String {
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

    /// Convert a key (string) to source code, adding quotes if necessary
    fn key_to_string(&self, key: &str) -> String {
        // if text does not just contain a-z, A-Z, 0-9, _, and starts with a-z, A-Z,  _, add quotes
        if !self.json_compat && is_alphanumeric_identifier(key) {
            key.to_string()
        } else {
            self.text_to_source_code(key)
        }
    }

    /// Convert a key (DatexExpression) to source code, adding parentheses if necessary
    fn key_expression_to_source_code(&self, key: &DatexExpression) -> String {
        match &key.data {
            DatexExpressionData::Text(t) => self.key_to_string(t),
            DatexExpressionData::Integer(i) => i.to_string(),
            DatexExpressionData::TypedInteger(ti) => {
                if self.add_variant_suffix() {
                    ti.to_string_with_suffix()
                } else {
                    ti.to_string()
                }
            }
            _ => format!("({})", self.format(key)),
        }
    }
    fn key_type_expression_to_source_code(
        &self,
        key: &TypeExpressionData,
    ) -> String {
        match key {
            TypeExpressionData::Text(t) => self.key_to_string(t),
            TypeExpressionData::Integer(i) => i.to_string(),
            TypeExpressionData::TypedInteger(ti) => {
                if self.add_variant_suffix() {
                    ti.to_string_with_suffix()
                } else {
                    ti.to_string()
                }
            }
            _ => format!("({})", self.type_expression_to_source_code(key)),
        }
    }

    /// Convert a TypeExpression to source code
    fn type_expression_to_source_code(
        &self,
        type_expr: &TypeExpressionData,
    ) -> String {
        match type_expr {
            TypeExpressionData::Integer(ti) => ti.to_string(),
            TypeExpressionData::Decimal(td) => td.to_string(),
            TypeExpressionData::Boolean(boolean) => boolean.to_string(),
            TypeExpressionData::Text(text) => text.to_string(),
            TypeExpressionData::Endpoint(endpoint) => endpoint.to_string(),
            TypeExpressionData::Null => "null".to_string(),
            TypeExpressionData::Ref(inner) => {
                format!("&{}", self.type_expression_to_source_code(inner,))
            }
            TypeExpressionData::RefMut(inner) => {
                format!("&mut {}", self.type_expression_to_source_code(inner,))
            }
            TypeExpressionData::RefFinal(inner) => {
                format!(
                    "&final {}",
                    self.type_expression_to_source_code(inner,)
                )
            }
            TypeExpressionData::Literal(literal) => literal.to_string(),
            TypeExpressionData::VariableAccess(VariableAccess {
                name, ..
            }) => name.to_string(),
            TypeExpressionData::GetReference(pointer_address) => {
                format!("{}", pointer_address) // FIXME #471
            }
            TypeExpressionData::TypedInteger(typed_integer) => {
                if self.add_variant_suffix() {
                    typed_integer.to_string_with_suffix()
                } else {
                    typed_integer.to_string()
                }
            }
            TypeExpressionData::TypedDecimal(typed_decimal) => {
                if self.add_variant_suffix() {
                    typed_decimal.to_string_with_suffix()
                } else {
                    typed_decimal.to_string()
                }
            }
            TypeExpressionData::StructuralList(type_expressions) => {
                let elements: Vec<String> = type_expressions
                    .iter()
                    .map(|e| self.type_expression_to_source_code(e))
                    .collect();
                self.wrap_list_elements(elements)
            }
            TypeExpressionData::FixedSizeList(type_expression, _) => todo!("#472 Undescribed by author."),
            TypeExpressionData::SliceList(type_expression) => todo!("#473 Undescribed by author."),
            TypeExpressionData::Intersection(type_expressions) => {
                let elements: Vec<String> = type_expressions
                    .iter()
                    .map(|e| self.type_expression_to_source_code(e))
                    .collect();
                self.wrap_intersection_elements(elements)
            }
            TypeExpressionData::Union(type_expressions) => {
                let elements: Vec<String> = type_expressions
                    .iter()
                    .map(|e| self.type_expression_to_source_code(e))
                    .collect();
                self.wrap_union_elements(elements)
            }
            TypeExpressionData::Generic(_, type_expressions) => todo!("#474 Undescribed by author."),
            TypeExpressionData::Function {
                parameters,
                return_type,
            } => {
                let params_code: Vec<String> = parameters
                    .iter()
                    .map(|(param_name, param_type)| {
                        ast_fmt!(
                            &self,
                            "{}:%s{}",
                            param_name,
                            self.type_expression_to_source_code(param_type,)
                        )
                    })
                    .collect();
                let return_type_code = format!(
                    "{}{}",
                    self.pad("->"),
                    self.type_expression_to_source_code(return_type)
                );
                ast_fmt!(
                    &self,
                    "({}){}",
                    params_code.join(&ast_fmt!(&self, ",%s")),
                    return_type_code
                )
            }
            TypeExpressionData::StructuralMap(items) => {
                let elements: Vec<String> = items
                    .iter()
                    .map(|(k, v)| {
                        format!(
                            "{}:{}{}",
                            self.key_type_expression_to_source_code(k),
                            if matches!(self.mode, FormattingMode::Compact) {
                                ""
                            } else {
                                " "
                            },
                            self.type_expression_to_source_code(v)
                        )
                    })
                    .collect();
                self.wrap_map_elements(elements)
            }
        }
    }

    fn wrap_map_elements(&self, elements: Vec<String>) -> String {
        self.wrap_elements(elements, BraceStyle::Curly, Some(","))
    }
    fn wrap_list_elements(&self, elements: Vec<String>) -> String {
        self.wrap_elements(elements, BraceStyle::Square, Some(","))
    }
    fn wrap_union_elements(&self, elements: Vec<String>) -> String {
        self.wrap_elements(elements, BraceStyle::None, Some("|"))
    }
    fn wrap_intersection_elements(&self, elements: Vec<String>) -> String {
        self.wrap_elements(elements, BraceStyle::None, Some("&"))
    }

    /// Wrap elements with commas and appropriate braces, handling pretty/compact modes
    fn wrap_elements(
        &self,
        elements: Vec<String>,
        brace_style: BraceStyle,
        separator: Option<&str>,
    ) -> String {
        let separator = separator.unwrap_or("");

        // Compact mode
        if matches!(self.mode, FormattingMode::Compact) {
            return format!(
                "{}{}{}",
                brace_style.open(),
                elements.join(separator),
                brace_style.close()
            );
        }

        // Pretty mode
        // decide separator in pretty mode
        let sep = format!("{}{}", separator, self.space());

        // If any element contains newline, force multiline
        let has_newline = elements.iter().any(|e| e.contains('\n'));

        let joined_inline = elements.join(&sep);
        let inline_len = brace_style.open().len()
            + joined_inline.len()
            + brace_style.close().len();

        if !has_newline && inline_len <= Self::MAX_INLINE {
            // single-line
            return format!(
                "{}{}{}",
                brace_style.open(),
                joined_inline,
                brace_style.close()
            );
        }

        // Multiline: build relative representation
        let unit = self.indent(); // one indent unit (e.g. "  ")

        let mut out = String::new();
        out.push_str(brace_style.open());
        out.push_str(self.newline());

        let elems_len = elements.len();
        for (i, elem) in elements.into_iter().enumerate() {
            // indent every line of the element by ONE unit inside this returned string
            // so inner multi-line elements keep their local structure.
            let indented = elem.replace("\n", &format!("\n{}", unit));
            out.push_str(unit.as_str());
            out.push_str(&indented);

            if i + 1 < elems_len {
                out.push_str(separator);
            }
            out.push_str(self.newline());
        }

        // closing brace at column 0 of this returned string (no base indent)
        out.push_str(brace_style.close());
        out
    }

    /// Convert a map (key/value pairs) to source code using join_elements.
    fn map_to_source_code(&self, map: &Map) -> String {
        let elements: Vec<String> = map
            .entries
            .iter()
            .map(|(k, v)| {
                // key -> source, colon, optional space (handled via self.space()), then formatted value
                format!(
                    "{}:{}{}",
                    self.key_expression_to_source_code(k),
                    if matches!(self.mode, FormattingMode::Compact) {
                        ""
                    } else {
                        " "
                    },
                    self.format(v)
                )
            })
            .collect();
        self.wrap_map_elements(elements)
    }

    /// Convert a list/array to source code.
    fn list_to_source_code(&self, list: &List) -> String {
        let elements: Vec<String> =
            list.items.iter().map(|v| self.format(v)).collect();
        self.wrap_list_elements(elements)
    }

    pub fn format(&self, ast: &DatexExpression) -> String {
        match &ast.data {
            DatexExpressionData::Integer(i) => i.to_string(),
            DatexExpressionData::TypedInteger(ti) => {
                if self.add_variant_suffix() {
                    ti.to_string_with_suffix()
                } else {
                    ti.to_string()
                }
            }
            DatexExpressionData::Decimal(d) => d.to_string(),
            DatexExpressionData::TypedDecimal(td) => {
                if self.add_variant_suffix() {
                    td.to_string_with_suffix()
                } else {
                    td.to_string()
                }
            }
            DatexExpressionData::Boolean(b) => b.to_string(),
            DatexExpressionData::Text(t) => self.text_to_source_code(t),
            DatexExpressionData::Endpoint(e) => e.to_string(),
            DatexExpressionData::Null => "null".to_string(),
            DatexExpressionData::Identifier(l) => l.to_string(),
            DatexExpressionData::Map(map) => self.map_to_source_code(map),
            DatexExpressionData::List(list) => self.list_to_source_code(list),
            DatexExpressionData::CreateRef(expr) => {
                format!("&{}", self.format(expr))
            }
            DatexExpressionData::CreateRefMut(expr) => {
                format!("&mut {}", self.format(expr))
            }
            DatexExpressionData::CreateRefFinal(expr) => {
                format!("&final {}", self.format(expr))
            }
            DatexExpressionData::BinaryOperation(BinaryOperation {
                operator,
                left,
                right,
                ..
            }) => {
                let left_code = self.key_expression_to_source_code(left);
                let right_code = self.key_expression_to_source_code(right);
                ast_fmt!(&self, "{}%s{}%s{}", left_code, operator, right_code)
            }
            DatexExpressionData::ApplyChain(ApplyChain {
                base,
                operations,
            }) => {
                let mut applies_code = vec![];
                for apply in operations {
                    match apply {
                        ApplyOperation::FunctionCall(args) => {
                            let args_code = self.format(args);
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
                                self.key_expression_to_source_code(prop)
                            ));
                        }
                        _ => todo!("#419 Undescribed by author."),
                    }
                }
                format!("{}{}", self.format(base), applies_code.join(""))
            }
            DatexExpressionData::TypeExpression(type_expr) => {
                format!(
                    "type({})",
                    self.type_expression_to_source_code(type_expr)
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
                        let code = self.format(stmt);
                        ast_fmt!(&self, "{};%n", code)
                    })
                    .collect();
                statements_code.join("")
            }
            DatexExpressionData::GetReference(pointer_address) => {
                format!("{}", pointer_address) // FIXME #475
            }
            DatexExpressionData::Conditional(Conditional {
                condition,
                then_branch,
                else_branch,
            }) => todo!("#476 Undescribed by author."),
            DatexExpressionData::VariableDeclaration(VariableDeclaration {
                id: _,
                kind,
                name,
                init_expression,
                type_annotation,
            }) => {
                let mut code = String::new();
                code.push_str(&kind.to_string());
                code.push(' ');
                code.push_str(name);
                if let Some(type_annotation) = type_annotation {
                    code.push_str(&ast_fmt!(&self, ":%s"));
                    code.push_str(
                        &self.type_expression_to_source_code(type_annotation),
                    );
                }
                code.push_str(&self.pad("="));
                code.push_str(&self.format(init_expression));
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
                code.push_str(&self.pad(&operator.to_string()));
                code.push_str(&self.format(expression));
                code
            }
            DatexExpressionData::VariableAccess(VariableAccess {
                name,
                ..
            }) => name.to_string(),
            DatexExpressionData::TypeDeclaration(TypeDeclaration {
                id: _,
                name,
                value,
                hoisted: _,
            }) => {
                ast_fmt!(
                    &self,
                    "type {}%s=%s{}",
                    name,
                    self.type_expression_to_source_code(value)
                )
            }
            DatexExpressionData::Type(type_expression) => {
                self.type_expression_to_source_code(type_expression)
            }
            DatexExpressionData::FunctionDeclaration(FunctionDeclaration {
                name,
                parameters,
                return_type,
                body,
            }) => {
                let params_code: Vec<String> = parameters
                    .iter()
                    .map(|(param_name, param_type)| {
                        ast_fmt!(
                            &self,
                            "{}:%s{}",
                            param_name,
                            self.type_expression_to_source_code(param_type,)
                        )
                    })
                    .collect();
                let return_type_code = if let Some(return_type) = return_type {
                    format!(
                        "{}{}",
                        self.pad("->"),
                        self.type_expression_to_source_code(return_type)
                    )
                } else {
                    "".to_string()
                };
                let body_code = self.format(body);
                ast_fmt!(
                    &self,
                    "fn {}({}){}%s(%n{}%n)",
                    name,
                    params_code.join(", "),
                    return_type_code,
                    body_code
                )
            }
            DatexExpressionData::Deref(datex_expression) => {
                format!("*{}", self.format(datex_expression))
            }
            DatexExpressionData::Slot(slot) => slot.to_string(),
            DatexExpressionData::SlotAssignment(SlotAssignment {
                slot,
                expression,
            }) => {
                format!("{}%s=%s{}", slot, self.format(expression))
            }
            DatexExpressionData::PointerAddress(pointer_address) => {
                pointer_address.to_string()
            }
            DatexExpressionData::ComparisonOperation(ComparisonOperation {
                operator,
                left,
                right,
            }) => {
                ast_fmt!(
                    &self,
                    "{}%s{operator}%s{}",
                    self.format(left),
                    self.format(right)
                )
            }
            DatexExpressionData::DerefAssignment(DerefAssignment {
                operator,
                deref_count,
                deref_expression,
                assigned_expression,
            }) => {
                let deref_prefix = "*".repeat(*deref_count);
                ast_fmt!(
                    &self,
                    "{}{}%s{operator}%s{}",
                    deref_prefix,
                    self.format(deref_expression),
                    self.format(assigned_expression)
                )
            }
            DatexExpressionData::UnaryOperation(unary_operation) => {
                format!(
                    "{}{}",
                    unary_operation.operator,
                    self.format(&unary_operation.expression)
                )
            }
            DatexExpressionData::Placeholder => "?".to_string(),
            DatexExpressionData::RemoteExecution(RemoteExecution {
                left,
                right,
            }) => {
                format!("{}%s::%s{}", self.format(left), self.format(right))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use indoc::indoc;

    use super::*;
    use crate::{
        ast::{
            assignment_operation::AssignmentOperator, parse, tree::VariableKind,
        },
        values::core_values::decimal::Decimal,
    };

    fn compact() -> AstToSourceCodeFormatter {
        AstToSourceCodeFormatter::new(FormattingMode::Compact, false, false)
    }

    fn pretty() -> AstToSourceCodeFormatter {
        AstToSourceCodeFormatter::new(FormattingMode::Pretty, false, false)
    }

    fn json() -> AstToSourceCodeFormatter {
        AstToSourceCodeFormatter::new(FormattingMode::Pretty, true, false)
    }

    fn to_expression(s: &str) -> DatexExpression {
        parse(s).unwrap().ast
    }

    #[test]
    fn nested_list() {
        let src = to_expression("[1, 2, 3]");
        assert_eq!(compact().format(&src), "[1,2,3]");
        assert_eq!(pretty().format(&src), "[1, 2, 3]");
        assert_eq!(json().format(&src), "[1, 2, 3]");

        let src = to_expression(
            "[1, [2, 3, 100, 200, 300, 400, 100, 200, 300, 100000000000000000000000000000000], 4]",
        );
        assert_eq!(
            compact().format(&src),
            "[1,[2,3,100,200,300,400,100,200,300,100000000000000000000000000000000],4]"
        );
        assert_eq!(
            pretty().format(&src),
            indoc! {
            "[
			   1,
			   [
			     2,
			     3,
			     100,
			     200,
			     300,
			     400,
			     100,
			     200,
			     300,
			     100000000000000000000000000000000
			   ],
			   4
			 ]"}
        );

        let src = to_expression(
            "[1, {a: 42, b: 100000000000, c: [1,2,3,1000000000000000000000000000]}, 3]",
        );
        assert_eq!(
            compact().format(&src),
            "[1,{a:42,b:100000000000,c:[1,2,3,1000000000000000000000000000]},3]"
        );
        assert_eq!(
            pretty().format(&src),
            indoc! {
            "[
			   1,
			   {
			     a: 42,
			     b: 100000000000,
			     c: [1, 2, 3, 1000000000000000000000000000]
			   },
			   3
			 ]"}
        );
    }

    #[test]
    fn test_primitives() {
        let int_ast = DatexExpressionData::Integer(42.into());
        assert_eq!(compact().format(&int_ast.with_default_span()), "42");

        let typed_int_ast = DatexExpressionData::TypedInteger(42i8.into());
        assert_eq!(compact().format(&typed_int_ast.with_default_span()), "42");

        let decimal_ast =
            DatexExpressionData::Decimal(Decimal::from_string("1.23").unwrap());
        assert_eq!(compact().format(&decimal_ast.with_default_span()), "1.23");

        let decimal_ast = DatexExpressionData::Decimal(Decimal::Infinity);
        assert_eq!(
            compact().format(&decimal_ast.with_default_span()),
            "infinity"
        );

        let decimal_ast = DatexExpressionData::Decimal(Decimal::NegInfinity);
        assert_eq!(
            compact().format(&decimal_ast.with_default_span()),
            "-infinity"
        );

        let decimal_ast = DatexExpressionData::Decimal(Decimal::NaN);
        assert_eq!(compact().format(&decimal_ast.with_default_span()), "nan");

        let typed_decimal_ast =
            DatexExpressionData::TypedDecimal(2.71f32.into());
        assert_eq!(
            pretty().format(&typed_decimal_ast.with_default_span()),
            "2.71f32"
        );

        let bool_ast = DatexExpressionData::Boolean(true);
        assert_eq!(compact().format(&bool_ast.with_default_span()), "true");

        let text_ast = DatexExpressionData::Text("Hello".to_string());
        assert_eq!(
            compact().format(&text_ast.with_default_span()),
            "\"Hello\""
        );

        let null_ast = DatexExpressionData::Null;
        assert_eq!(compact().format(&null_ast.with_default_span()), "null");
    }

    #[test]
    fn test_list() {
        let list_ast = DatexExpressionData::List(List::new(vec![
            DatexExpressionData::Integer(1.into()).with_default_span(),
            DatexExpressionData::Integer(2.into()).with_default_span(),
            DatexExpressionData::Integer(3.into()).with_default_span(),
        ]));
        assert_eq!(compact().format(&list_ast.with_default_span()), "[1,2,3]");

        // long list should be multi-line
        let long_list_ast = DatexExpressionData::List(List::new(vec![
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
        ]));

        assert_eq!(
            pretty().format(&long_list_ast.with_default_span()),
            indoc! {
            "[
			   \"This is a long string\",
			   \"Another long string\",
			   \"Yet another long string\",
			   \"More long strings to increase length\",
			   \"Final long string in the list\"
			 ]"}
        );
    }

    #[test]
    fn test_map() {
        let map_ast = DatexExpressionData::Map(Map::new(vec![
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
            (
                DatexExpressionData::Text("x".repeat(30).to_string())
                    .with_default_span(),
                DatexExpressionData::Integer(42.into()).with_default_span(),
            ),
        ]))
        .with_default_span();
        assert_eq!(
            compact().format(&map_ast),
            "{key1:1,key2:\"two\",42:true,xxxxxxxxxxxxxxxxxxxxxxxxxxxxxx:42}"
        );
        assert_eq!(
            pretty().format(&map_ast),
            indoc! {
            "{
			   key1: 1,
			   key2: \"two\",
			   42: true,
			   xxxxxxxxxxxxxxxxxxxxxxxxxxxxxx: 42
			 }"}
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
        assert_eq!(compact().format(&deref_ast.with_default_span()), "*ptr");
    }

    #[test]
    fn test_deref_assignment() {
        let deref_assign_ast =
            DatexExpressionData::DerefAssignment(DerefAssignment {
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
            });
        assert_eq!(
            compact().format(&deref_assign_ast.with_default_span()),
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
                    DatexExpressionData::TypedInteger(10u8.into())
                        .with_default_span(),
                ),
                type_annotation: Some(TypeExpressionData::RefMut(Box::new(
                    TypeExpressionData::Literal("integer/u8".to_owned()),
                ))),
            })
            .with_default_span();
        assert_eq!(
            compact().format(&var_decl_ast),
            "const x:&mut integer/u8=10"
        );
        assert_eq!(
            pretty().format(&var_decl_ast),
            "const x: &mut integer/u8 = 10u8"
        );
    }

    #[test]
    fn typed_variants() {
        let typed_int_ast =
            DatexExpressionData::TypedInteger(42i8.into()).with_default_span();
        assert_eq!(pretty().format(&typed_int_ast), "42i8");
        assert_eq!(json().format(&typed_int_ast), "42");

        let typed_decimal_ast =
            DatexExpressionData::TypedDecimal(2.71f32.into())
                .with_default_span();
        assert_eq!(pretty().format(&typed_decimal_ast), "2.71f32");
        assert_eq!(json().format(&typed_decimal_ast), "2.71");
    }
}
