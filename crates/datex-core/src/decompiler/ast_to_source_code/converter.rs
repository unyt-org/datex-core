use crate::{
    ast::{
        expressions::{
            Apply, BinaryOperation, CallableDeclaration, ComparisonOperation,
            Conditional, DatexExpression, DatexExpressionData, List, Map,
            PropertyAccess, PropertyAssignment, RangeDeclaration,
            RemoteExecution, StackAssignment, TypeDeclarationExpression,
            UnboxAssignment, VariableAccess, VariableAssignment,
            VariableDeclaration, VariantAccess,
        },
        type_expressions::{
            CallableTypeExpression, TypeExpression, TypeExpressionData,
            TypeVariantAccess,
        },
    },
    values::core_values::text::Text,
};

use crate::{
    ast::{
        expressions::{
            CallableSignature, EntityDeclarationExpression,
            InterfaceMethodCall, RootPropertyAccess, StackListAssignment,
            UnboxSlotAssignment, ValueAccessType,
        },
        type_expressions::IdentifierWithPointerAddress,
    },
    decompiler::{FormattingMode, FormattingOptions, IndentType},
    prelude::*,
    shared_values::ReferenceMutability,
    types::type_definition_with_metadata::LocalReferenceMutability,
};
use alloc::format;
use core::{
    fmt::{self},
    ops::Deref,
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

pub struct AstToSourceCodeConverter {
    options: FormattingOptions,
}

#[macro_export]
macro_rules! ast_fmt {
    ($fmtter:expr, $fmt:expr $(, $args:expr )* $(,)?) => {
        $fmtter.fmt(core::format_args!($fmt $(, $args )*))
    };
}
impl AstToSourceCodeConverter {
    const MAX_INLINE: usize = 60;

    pub fn new(options: FormattingOptions) -> Self {
        Self { options }
    }

    /// Whether to add type variant suffixes to typed integers and decimals
    fn add_variant_suffix(&self) -> bool {
        !self.options.json_compat
    }

    /// Return the indentation as a string
    fn indent(&self) -> String {
        match &self.options.mode {
            FormattingMode::Pretty {
                indent_type,
                indent,
            } => {
                let char = match indent_type {
                    IndentType::Spaces => &' ',
                    IndentType::Tabs => &'\t',
                };
                char.to_string().repeat(*indent)
            }
            FormattingMode::Compact => "".into(),
        }
    }

    fn is_compact_mode(&self) -> bool {
        core::matches!(self.options.mode, FormattingMode::Compact)
    }

    /// Return a space or empty string based on formatting mode
    fn space(&self) -> &'static str {
        if self.is_compact_mode() { "" } else { " " }
    }

    // Return a newline or empty string based on formatting mode
    fn newline(&self) -> &'static str {
        if self.is_compact_mode() { "" } else { "\n" }
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
        if self.is_compact_mode() {
            s.to_string()
        } else {
            format!("{}{}{}", self.space(), s, self.space())
        }
    }

    /// Escape text to be a valid source code string literal
    fn text_to_source_code(&self, text: &Text) -> String {
        let text = text
            .as_str()
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
    fn key_to_string(&self, key: &Text) -> String {
        // if text does not just contain a-z, A-Z, 0-9, _, and starts with a-z, A-Z,  _, add quotes
        if !self.options.json_compat
            && is_alphanumeric_identifier(key.as_str())
        {
            key.to_string()
        } else {
            self.text_to_source_code(key)
        }
    }

    /// Convert a key (DatexExpression) to source code, adding parentheses if necessary
    fn key_expression_to_source_code(&self, key: &DatexExpression) -> String {
        match key.data() {
            DatexExpressionData::Text(t) => self.key_to_string(t),
            DatexExpressionData::Integer(i) => i.to_string(),
            DatexExpressionData::TypedInteger(ti) => {
                if self.add_variant_suffix() {
                    ti.to_string_with_suffix()
                } else {
                    ti.to_string()
                }
            }
            _ => format!("({})", self.format_child(key)),
        }
    }
    fn key_type_expression_to_source_code(
        &self,
        key: &TypeExpression,
    ) -> String {
        match key.data() {
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
        type_expr: &TypeExpression,
    ) -> String {
        match type_expr.data() {
            TypeExpressionData::VariantAccess(TypeVariantAccess {
                name,
                variant,
                ..
            }) => {
                format!("{}/{}", name, variant)
            }
            TypeExpressionData::Integer(ti) => ti.to_string(),
            TypeExpressionData::Decimal(td) => td.to_string(),
            TypeExpressionData::Boolean(boolean) => boolean.to_string(),
            TypeExpressionData::Text(text) => self.text_to_source_code(text),
            TypeExpressionData::Endpoint(endpoint) => endpoint.to_string(),
            TypeExpressionData::Ref(inner) => {
                format!("&{}", self.type_expression_to_source_code(inner,))
            }
            TypeExpressionData::RefMut(inner) => {
                format!("&mut {}", self.type_expression_to_source_code(inner,))
            }
            TypeExpressionData::Shared(inner) => {
                format!(
                    "shared {}",
                    self.type_expression_to_source_code(inner,)
                )
            }
            TypeExpressionData::Mut(inner) => {
                format!("mut {}", self.type_expression_to_source_code(inner,))
            }
            TypeExpressionData::Identifier(literal) => literal.to_string(),
            TypeExpressionData::IdentifierWithPointerAddress(
                IdentifierWithPointerAddress {
                    name,
                    pointer_address: _,
                },
            ) => {
                // TODO: also print pointer address in debug mode
                // format!("{}{}", name, pointer_address)
                name.clone()
            }
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
                    .0
                    .iter()
                    .map(|e| self.type_expression_to_source_code(e))
                    .collect();
                self.wrap_list_elements(elements)
            }
            TypeExpressionData::Range(type_expr) => {
                let x = self.type_expression_to_source_code(&type_expr.start);
                let y = self.type_expression_to_source_code(&type_expr.end);
                ast_fmt!(&self, "{}..{}", x, y)
            }
            TypeExpressionData::FixedSizeList(_fixed_size_list) => {
                core::todo!("#472 Undescribed by author.")
            }
            TypeExpressionData::SliceList(_type_expression) => {
                core::todo!("#473 Undescribed by author.")
            }
            TypeExpressionData::Intersection(type_expressions) => {
                let elements: Vec<String> = type_expressions
                    .0
                    .iter()
                    .map(|e| self.type_expression_to_source_code(e))
                    .collect();
                self.wrap_intersection_elements(elements)
            }
            TypeExpressionData::Union(type_expressions) => {
                let elements: Vec<String> = type_expressions
                    .0
                    .iter()
                    .map(|e| self.type_expression_to_source_code(e))
                    .collect();
                self.wrap_union_elements(elements)
            }
            TypeExpressionData::GenericAccess(_generic_access) => {
                core::todo!("#474 Undescribed by author.")
            }
            TypeExpressionData::Callable(CallableTypeExpression {
                kind,
                parameter_types,
                rest_parameter_type,
                return_type,
                yeet_type,
            }) => {
                let mut params_code: Vec<String> = parameter_types
                    .iter()
                    .map(|(param_name, param_type)| match param_name {
                        Some(name) => ast_fmt!(
                            &self,
                            "{}:%s{}",
                            name,
                            self.type_expression_to_source_code(param_type)
                        ),
                        None => self.type_expression_to_source_code(param_type),
                    })
                    .collect();

                // handle rest parameter
                if let Some((param_name, param_type)) = rest_parameter_type {
                    params_code.push(match param_name {
                        Some(name) => ast_fmt!(
                            &self,
                            "...{}:%s{}",
                            name,
                            self.type_expression_to_source_code(param_type)
                        ),
                        None => ast_fmt!(
                            &self,
                            "...{}",
                            self.type_expression_to_source_code(param_type)
                        ),
                    });
                }

                let return_type_code = match return_type {
                    Some(return_type) => format!(
                        "{}{}",
                        " -> ",
                        self.type_expression_to_source_code(return_type)
                    ),
                    None => format!("{}()", " -> ").to_string(),
                };

                let yeet_type_code = match yeet_type {
                    Some(yeet_type) => format!(
                        " yeets {}",
                        self.type_expression_to_source_code(yeet_type)
                    ),
                    None => "".to_string(),
                };

                ast_fmt!(
                    &self,
                    "{} ({}){}{}",
                    kind,
                    params_code.join(&ast_fmt!(&self, ",%s")),
                    return_type_code,
                    yeet_type_code
                )
            }
            TypeExpressionData::StructuralMap(items) => {
                let elements: Vec<String> = items
                    .0
                    .iter()
                    .map(|(k, v)| {
                        format!(
                            "{}:{}{}",
                            self.key_type_expression_to_source_code(k),
                            self.space(),
                            self.type_expression_to_source_code(v)
                        )
                    })
                    .collect();
                self.wrap_map_elements(elements)
            }
            TypeExpressionData::Recover => {
                // TODO #653
                "/*Recovered Type Expression*/".to_string()
            }
            TypeExpressionData::GetCoreLibType(core_id) => core_id.to_string(),
        }
    }

    fn wrap_map_elements(&self, elements: Vec<String>) -> String {
        self.wrap_elements(elements, BraceStyle::Curly, Some(","), false)
    }
    fn wrap_list_elements(&self, elements: Vec<String>) -> String {
        self.wrap_elements(elements, BraceStyle::Square, Some(","), false)
    }
    fn wrap_statement_elements(&self, elements: Vec<String>) -> String {
        self.wrap_elements(elements, BraceStyle::Paren, None, true)
    }
    fn wrap_union_elements(&self, elements: Vec<String>) -> String {
        self.wrap_elements(elements, BraceStyle::None, Some("|"), false)
    }
    fn wrap_intersection_elements(&self, elements: Vec<String>) -> String {
        self.wrap_elements(elements, BraceStyle::None, Some("&"), false)
    }

    /// Wrap elements with commas and appropriate braces, handling pretty/compact modes
    fn wrap_elements(
        &self,
        elements: Vec<String>,
        brace_style: BraceStyle,
        separator: Option<&str>,
        force_newlines: bool,
    ) -> String {
        let separator = separator.unwrap_or("");

        // Compact mode
        if self.is_compact_mode() {
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

        if !has_newline && inline_len <= Self::MAX_INLINE && !force_newlines {
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
                    self.space(),
                    self.format_child(v)
                )
            })
            .collect();
        self.wrap_map_elements(elements)
    }

    /// Convert a list/array to source code.
    fn list_to_source_code(&self, list: &List) -> String {
        let elements: Vec<String> =
            list.items.iter().map(|v| self.format_child(v)).collect();
        self.wrap_list_elements(elements)
    }

    /// Format a DatexExpression tree into source code
    pub fn format(&self, ast: &DatexExpression) -> String {
        self.format_inner(ast, true)
    }

    /// Format a DatexExpression tree into source code, treating it as a child node (not the root).
    fn format_child(&self, ast: &DatexExpression) -> String {
        self.format_inner(ast, false)
    }

    /// Format a DatexExpression tree into source code, with an option to indicate if it's the root node.
    fn format_inner(&self, ast: &DatexExpression, is_root: bool) -> String {
        match ast.data() {
            DatexExpressionData::PropertyAssignment(PropertyAssignment {
                operator,
                base,
                assigned_expression,
                property,
            }) => {
                ast_fmt!(
                    &self,
                    "{}.{}{}%s{}%s{}",
                    self.format_child(base),
                    self.key_expression_to_source_code(property),
                    self.space(),
                    match operator {
                        Some(operator) => operator.to_string(),
                        None => "=".to_string(),
                    },
                    self.format_child(assigned_expression)
                )
            }
            DatexExpressionData::VariantAccess(VariantAccess {
                name,
                variant,
                ..
            }) => {
                format!("{}/{}", name, variant)
            }
            DatexExpressionData::Noop => "".to_string(),
            DatexExpressionData::Integer(i) => i.to_string(),
            DatexExpressionData::Instant(instant) => instant.to_iso_string(),
            DatexExpressionData::Range(RangeDeclaration { start, end }) => {
                let left_code = self.format_child(start);
                let right_code = self.format_child(end);
                ast_fmt!(&self, "{}..{}", left_code, right_code)
            }
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
            DatexExpressionData::OmitRecursive => "(...)".to_string(),
            DatexExpressionData::DeriveRef(create_ref) => {
                match &create_ref.mutability {
                    LocalReferenceMutability::Mutable => {
                        format!(
                            "&mut {}",
                            self.format_child(&create_ref.expression)
                        )
                    }
                    LocalReferenceMutability::Immutable => {
                        format!(
                            "&{}",
                            self.format_child(&create_ref.expression)
                        )
                    }
                }
            }
            DatexExpressionData::EntityValue(entity_value) => {
                format!(
                    "{} {}",
                    entity_value.entity_name,
                    self.format_child(&entity_value.value)
                )
            }
            DatexExpressionData::DeriveSharedRef(create_shared_ref) => {
                match &create_shared_ref.mutability {
                    ReferenceMutability::Mutable => {
                        format!(
                            "'mut {}",
                            self.format_child(&create_shared_ref.expression)
                        )
                    }
                    ReferenceMutability::Immutable => {
                        format!(
                            "'{}",
                            self.format_child(&create_shared_ref.expression)
                        )
                    }
                }
            }
            DatexExpressionData::CreateShared(create_shared) => {
                let mut mut_string = create_shared.mutability.to_string();
                if !mut_string.is_empty() {
                    mut_string.push(' ');
                }
                format!(
                    "shared {}{}",
                    mut_string,
                    self.format_child(&create_shared.expression)
                )
            }
            DatexExpressionData::CreateMut(create_shared) => {
                format!("mut {}", self.format_child(&create_shared.expression))
            }
            DatexExpressionData::MoveSharedValue(move_shared_value) => {
                format!("{}", move_shared_value)
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
            DatexExpressionData::Apply(Apply { base, arguments }) => {
                let mut args_source = vec![];

                for arg in arguments {
                    args_source.push(self.format_child(arg));
                }

                format!(
                    "{}({})",
                    self.format_child(base),
                    args_source.join(",")
                )
            }
            DatexExpressionData::InterfaceMethodCall(InterfaceMethodCall {
                target: base,
                method_name,
                arguments,
            }) => {
                let mut args_source = vec![];

                for arg in arguments {
                    args_source.push(self.format_child(arg));
                }

                format!(
                    "{}->{}({})",
                    self.format_child(base),
                    method_name,
                    args_source.join(",")
                )
            }
            DatexExpressionData::TypeExpression(type_expr) => {
                format!(
                    "type<{}>",
                    self.type_expression_to_source_code(type_expr)
                )
            }
            DatexExpressionData::Compile(compile_expr) => {
                ast_fmt!(
                    &self,
                    "compile (%n{}%n)",
                    self.format_child(&compile_expr.expression)
                )
            }
            DatexExpressionData::Recover => unreachable!(
                "DatexExpressionData::Recover should not appear in a valid AST"
            ),
            DatexExpressionData::Statements(statements) => {
                let terminated = statements.is_terminated;
                let statements_code: Vec<String> = statements
                    .statements
                    .iter()
                    .enumerate()
                    .map(|(i, stmt)| {
                        let code = self.format_child(stmt);
                        let is_last_statement =
                            i + 1 == statements.statements.len();
                        if !is_last_statement || terminated {
                            ast_fmt!(&self, "{};", code)
                        } else {
                            code
                        }
                    })
                    .collect();

                // no outer parentheses for root statements
                if is_root {
                    statements_code.join(self.newline())
                } else {
                    self.wrap_statement_elements(statements_code)
                }
            }
            DatexExpressionData::RequestSharedRef(get_shared_ref) => {
                format!(
                    "{}{}",
                    match get_shared_ref.mutability {
                        ReferenceMutability::Mutable => "'mut ",
                        ReferenceMutability::Immutable => "'",
                    },
                    get_shared_ref.address,
                )
            }
            DatexExpressionData::Conditional(Conditional {
                condition,
                then_branch,
                else_branch,
            }) => {
                let mut code = String::from("if (");
                code.push_str(&self.format(condition));
                code.push_str(") (");
                code.push_str(&self.format(then_branch));
                code.push(')');
                if let Some(else_branch) = else_branch.as_ref() {
                    code.push_str(" else (");
                    code.push_str(&self.format(else_branch));
                    code.push(')');
                }
                code
            }
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
                code.push_str(&self.format_child(init_expression));
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
                if let Some(operator) = operator {
                    code.push_str(&self.pad(&operator.to_string()));
                }
                code.push_str(&self.format_child(expression));
                code
            }
            DatexExpressionData::VariableAccess(VariableAccess {
                name,
                ..
            }) => name.to_string(),
            DatexExpressionData::TypeDeclaration(
                TypeDeclarationExpression {
                    name,
                    definition: value,
                    ..
                },
            ) => {
                ast_fmt!(
                    &self,
                    "type {}%s=%s{}",
                    name,
                    self.type_expression_to_source_code(value)
                )
            }
            DatexExpressionData::EntityDeclaration(
                EntityDeclarationExpression {
                    name,
                    definition: value,
                    ..
                },
            ) => {
                ast_fmt!(
                    &self,
                    "entity {}%s=%s{}",
                    name,
                    self.type_expression_to_source_code(value)
                )
            }
            DatexExpressionData::CallableDeclaration(callable) => {
                let CallableDeclaration {
                    signature:
                        CallableSignature {
                            name,
                            kind,
                            requires_async: _,
                            parameters,
                            rest_parameter,
                            return_type,
                            yeet_type,
                        },
                    body,
                    ..
                } = callable;
                let mut params_code: Vec<String> = parameters
                    .iter()
                    .map(|(param_name, param_type)| {
                        ast_fmt!(
                            &self,
                            "{}:%s{}",
                            param_name,
                            self.type_expression_to_source_code(param_type)
                        )
                    })
                    .collect();

                // handle rest parameter
                if let Some((param_name, param_type)) = rest_parameter {
                    params_code.push(ast_fmt!(
                        &self,
                        "...{}:%s{}[]",
                        param_name,
                        self.type_expression_to_source_code(param_type)
                    ));
                }

                let return_type_code = match return_type {
                    Some(return_type) => format!(
                        " -> {}",
                        self.type_expression_to_source_code(return_type)
                    ),
                    None => "".to_string(),
                };

                let yeet_type_code = match yeet_type {
                    Some(yeet_type) => format!(
                        " yeets {}",
                        self.type_expression_to_source_code(yeet_type)
                    ),
                    None => "".to_string(),
                };

                // indented function body
                let body_code = match body.data.deref() {
                    DatexExpressionData::NativeImplementationIndicator => {
                        "".to_string()
                    }
                    _ => {
                        let inner = self
                            .format_child(body)
                            .replace("\n", &format!("\n{}", self.indent()));
                        format!("%s(%n{}{}%n)", self.indent(), inner)
                    }
                };

                ast_fmt!(
                    &self,
                    "{} {}({}){}{}{}",
                    kind,
                    name.clone().unwrap_or_else(|| "".to_string()),
                    params_code.join(&ast_fmt!(&self, ", ")),
                    return_type_code,
                    yeet_type_code,
                    body_code
                )
            }
            DatexExpressionData::Unbox(unbox) => {
                format!("*{}", self.format_child(&unbox.expression))
            }
            DatexExpressionData::Clone(clone) => {
                format!("clone {}", self.format_child(&clone.expression))
            }
            DatexExpressionData::StackIndex(slot) => slot.to_string(),
            DatexExpressionData::StackAssignment(StackAssignment {
                index: slot,
                expression,
            }) => {
                ast_fmt!(
                    &self,
                    "{}%s=%s{}",
                    slot,
                    self.format_child(expression)
                )
            }
            DatexExpressionData::StackListAssignment(StackListAssignment {
                expression,
            }) => {
                ast_fmt!(&self, "{{...}}%s=%s{}", self.format_child(expression))
            }
            DatexExpressionData::ComparisonOperation(ComparisonOperation {
                operator,
                left,
                right,
            }) => {
                ast_fmt!(
                    &self,
                    "{}%s{operator}%s{}",
                    self.format_child(left),
                    self.format_child(right)
                )
            }
            DatexExpressionData::UnboxAssignment(UnboxAssignment {
                operator,
                unbox_expression,
                assigned_expression,
            }) => {
                ast_fmt!(
                    &self,
                    "{}%s{}%s{}",
                    self.format_child(unbox_expression),
                    match operator {
                        Some(operator) => operator.to_string(),
                        None => "=".to_string(),
                    },
                    self.format_child(assigned_expression)
                )
            }
            DatexExpressionData::UnboxSlotAssignment(UnboxSlotAssignment {
                operator,
                stack_index: slot,
                assigned_expression,
            }) => {
                let unbox_prefix = "*";
                ast_fmt!(
                    &self,
                    "{}{}%s{}%s{}",
                    unbox_prefix,
                    slot.to_string(),
                    match operator {
                        Some(operator) => operator.to_string(),
                        None => "=".to_string(),
                    },
                    self.format_child(assigned_expression)
                )
            }
            DatexExpressionData::UnaryOperation(unary_operation) => {
                format!(
                    "{}{}",
                    unary_operation.operator,
                    self.format_child(&unary_operation.expression)
                )
            }
            DatexExpressionData::Placeholder(placeholder_type) => {
                match placeholder_type {
                    ValueAccessType::SharedRef => "'?".to_string(),
                    ValueAccessType::SharedRefMut => "'mut ?".to_string(),
                    ValueAccessType::MoveOrCopy => "?".to_string(),
                    ValueAccessType::Clone => "clone ?".to_string(),
                    ValueAccessType::Borrow => "?".to_string(),
                }
            }
            DatexExpressionData::RemoteExecution(RemoteExecution {
                left,
                right,
                ..
            }) => {
                ast_fmt!(
                    &self,
                    "{}%s::%s{}",
                    self.format_child(left),
                    self.format_child(right)
                )
            }
            DatexExpressionData::PropertyAccess(PropertyAccess {
                base,
                property,
            }) => {
                format!(
                    "{}.{}",
                    self.format_child(base),
                    self.key_expression_to_source_code(property)
                )
            }
            DatexExpressionData::NativeImplementationIndicator => {
                "[[ native code ]]".to_string()
            }
            DatexExpressionData::GenericInstantiation(_) => {
                todo!("#654 Undescribed by author.")
            }
            DatexExpressionData::Tag(tag) => match &tag.expression {
                Some(tag_expression) => match tag_expression.data() {
                    DatexExpressionData::Map(_)
                    | DatexExpressionData::List(_)
                    | DatexExpressionData::Statements(_) => {
                        ast_fmt!(
                            &self,
                            "#{}%s{}",
                            tag.tag,
                            self.format_child(tag_expression)
                        )
                    }
                    _ => {
                        ast_fmt!(
                            &self,
                            "#{}%s({})",
                            tag.tag,
                            self.format_child(tag_expression)
                        )
                    }
                },
                None => format!("#{}", tag.tag),
            },
            DatexExpressionData::RootPropertyAccess(RootPropertyAccess {
                property_name,
            }) => {
                format!("$.{}", property_name)
            }
            DatexExpressionData::ResolveCoreLibId(core_lib_id) => {
                format!("{}", core_lib_id)
            }
        }
    }
}

#[cfg(test)]
#[cfg(feature = "compiler")]
mod tests {
    use crate::prelude::*;
    use indoc::indoc;

    use super::AstToSourceCodeConverter;
    use crate::{
        ast::{
            expressions::{
                Apply, DatexExpression, DatexExpressionData, List, Map,
                PropertyAccess, RangeDeclaration, Unbox, UnboxAssignment,
                ValueAccessType, VariableAccess, VariableDeclaration,
                VariableKind,
            },
            spanned::Spanned,
            type_expressions::TypeExpressionData,
        },
        decompiler::options::FormattingOptions,
        global::operators::ModificationOperator,
        parser::Parser,
        values::core_values::decimal::Decimal,
    };

    fn compact() -> AstToSourceCodeConverter {
        AstToSourceCodeConverter::new(FormattingOptions::compact())
    }

    fn pretty() -> AstToSourceCodeConverter {
        AstToSourceCodeConverter::new(FormattingOptions::pretty())
    }

    fn json_compat() -> AstToSourceCodeConverter {
        AstToSourceCodeConverter::new(FormattingOptions::json_compat())
    }

    fn to_expression(s: &str) -> DatexExpression {
        Parser::parse_with_default_options(s).unwrap()
    }

    #[test]
    fn nested_list() {
        let src = to_expression("[1, 2, 3]");
        assert_eq!(compact().format(&src), "[1,2,3]");
        assert_eq!(pretty().format(&src), "[1, 2, 3]");
        assert_eq!(json_compat().format(&src), "[1,2,3]");

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
    fn primitives() {
        let int_ast = DatexExpressionData::Integer(42.into());
        assert_eq!(compact().format(&int_ast.with_default_span()), "42");

        let typed_int_ast = DatexExpressionData::TypedInteger(42i8.into());
        assert_eq!(
            compact().format(&typed_int_ast.with_default_span()),
            "42i8"
        );

        let decimal_ast = DatexExpressionData::Decimal(
            Decimal::try_from_string("1.23").unwrap(),
        );
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

        let decimal_ast = DatexExpressionData::Decimal(Decimal::Nan);
        assert_eq!(compact().format(&decimal_ast.with_default_span()), "nan");

        let typed_decimal_ast =
            DatexExpressionData::TypedDecimal(2.71f32.into());
        assert_eq!(
            pretty().format(&typed_decimal_ast.with_default_span()),
            "2.71f32"
        );

        let bool_ast = DatexExpressionData::Boolean(true.into());
        assert_eq!(compact().format(&bool_ast.with_default_span()), "true");

        let text_ast = DatexExpressionData::Text("Hello".into());
        assert_eq!(
            compact().format(&text_ast.with_default_span()),
            "\"Hello\""
        );

        let null_ast = DatexExpressionData::Null;
        assert_eq!(compact().format(&null_ast.with_default_span()), "null");

        let range_ast = DatexExpressionData::Range(RangeDeclaration {
            start: (DatexExpressionData::Integer(11.into())
                .with_default_span()),
            end: (DatexExpressionData::Integer(13.into()).with_default_span()),
        });

        assert_eq!(compact().format(&range_ast.with_default_span()), "11..13");
    }

    #[test]
    fn list() {
        let list_ast = DatexExpressionData::List(List::new(vec![
            DatexExpressionData::Integer(1.into()).with_default_span(),
            DatexExpressionData::Integer(2.into()).with_default_span(),
            DatexExpressionData::Integer(3.into()).with_default_span(),
        ]));
        assert_eq!(compact().format(&list_ast.with_default_span()), "[1,2,3]");

        // long list should be multi-line
        let long_list_ast = DatexExpressionData::List(List::new(vec![
            DatexExpressionData::Text("This is a long string".into())
                .with_default_span(),
            DatexExpressionData::Text("Another long string".into())
                .with_default_span(),
            DatexExpressionData::Text("Yet another long string".into())
                .with_default_span(),
            DatexExpressionData::Text(
                "More long strings to increase length".into(),
            )
            .with_default_span(),
            DatexExpressionData::Text("Final long string in the list".into())
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
    fn map() {
        let map_ast = DatexExpressionData::Map(Map::new(vec![
            (
                DatexExpressionData::Text("key1".into()).with_default_span(),
                DatexExpressionData::Integer(1.into()).with_default_span(),
            ),
            (
                DatexExpressionData::Text("key2".into()).with_default_span(),
                DatexExpressionData::Text("two".into()).with_default_span(),
            ),
            (
                DatexExpressionData::Integer(42.into()).with_default_span(),
                DatexExpressionData::Boolean(true.into()).with_default_span(),
            ),
            (
                DatexExpressionData::Text("x".repeat(30).into())
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
    fn unbox() {
        let unbox_ast = DatexExpressionData::Unbox(Unbox {
            expression: (DatexExpressionData::VariableAccess(VariableAccess {
                id: 0,
                name: "ptr".to_string(),
                access_type: ValueAccessType::MoveOrCopy,
            })
            .with_default_span()),
        });
        assert_eq!(compact().format(&unbox_ast.with_default_span()), "*ptr");
    }

    #[test]
    fn unbox_assignment() {
        let unbox_assign_ast =
            DatexExpressionData::UnboxAssignment(UnboxAssignment {
                operator: Some(ModificationOperator::AddAssign),
                unbox_expression: (DatexExpressionData::VariableAccess(
                    VariableAccess {
                        id: 0,
                        name: "ptr".to_string(),
                        access_type: ValueAccessType::MoveOrCopy,
                    },
                )
                .with_default_span()),
                assigned_expression: (DatexExpressionData::Integer(42.into())
                    .with_default_span()),
            });
        assert_eq!(
            compact().format(&unbox_assign_ast.with_default_span()),
            "ptr+=42"
        );
    }

    #[test]
    fn variable_declaration() {
        let var_decl_ast =
            DatexExpressionData::VariableDeclaration(VariableDeclaration {
                id: Some(0),
                kind: VariableKind::Const,
                name: "x".to_string(),
                init_expression: (DatexExpressionData::TypedInteger(
                    10u8.into(),
                )
                .with_default_span()),
                type_annotation: Some(
                    TypeExpressionData::RefMut(Box::new(
                        TypeExpressionData::Identifier("integer/u8".to_owned())
                            .with_default_span(),
                    ))
                    .with_default_span(),
                ),
            })
            .with_default_span();
        assert_eq!(
            compact().format(&var_decl_ast),
            "const x:&mut integer/u8=10u8"
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
        assert_eq!(json_compat().format(&typed_int_ast), "42");

        let typed_decimal_ast =
            DatexExpressionData::TypedDecimal(2.71f32.into())
                .with_default_span();
        assert_eq!(pretty().format(&typed_decimal_ast), "2.71f32");
        assert_eq!(json_compat().format(&typed_decimal_ast), "2.71");
    }

    #[test]
    fn property_access_text_key() {
        let prop_access_ast =
            DatexExpressionData::PropertyAccess(PropertyAccess {
                base: (DatexExpressionData::VariableAccess(VariableAccess {
                    id: 0,
                    name: "obj".to_string(),
                    access_type: ValueAccessType::MoveOrCopy,
                })
                .with_default_span()),
                property: (DatexExpressionData::Text("myProp".into())
                    .with_default_span()),
            })
            .with_default_span();
        assert_eq!(compact().format(&prop_access_ast), "obj.myProp");
        assert_eq!(pretty().format(&prop_access_ast), "obj.myProp");
    }

    #[test]
    fn property_access_boolean_key() {
        let prop_access_ast =
            DatexExpressionData::PropertyAccess(PropertyAccess {
                base: (DatexExpressionData::VariableAccess(VariableAccess {
                    id: 0,
                    name: "obj".to_string(),
                    access_type: ValueAccessType::MoveOrCopy,
                })
                .with_default_span()),
                property: (DatexExpressionData::Boolean(true.into())
                    .with_default_span()),
            })
            .with_default_span();
        assert_eq!(compact().format(&prop_access_ast), "obj.(true)");
        assert_eq!(pretty().format(&prop_access_ast), "obj.(true)");
    }

    #[test]
    fn property_access_integer_key() {
        let prop_access_ast =
            DatexExpressionData::PropertyAccess(PropertyAccess {
                base: (DatexExpressionData::VariableAccess(VariableAccess {
                    id: 0,
                    name: "obj".to_string(),
                    access_type: ValueAccessType::MoveOrCopy,
                })
                .with_default_span()),
                property: (DatexExpressionData::Integer(42.into())
                    .with_default_span()),
            })
            .with_default_span();
        assert_eq!(compact().format(&prop_access_ast), "obj.42");
        assert_eq!(pretty().format(&prop_access_ast), "obj.42");
    }

    #[test]
    fn apply_single_argument() {
        let apply_ast = DatexExpressionData::Apply(Apply {
            base: (DatexExpressionData::VariableAccess(VariableAccess {
                id: 0,
                name: "func".to_string(),
                access_type: ValueAccessType::MoveOrCopy,
            })
            .with_default_span()),
            arguments: vec![
                DatexExpressionData::Integer(10.into()).with_default_span(),
            ],
        })
        .with_default_span();
        assert_eq!(compact().format(&apply_ast), "func(10)");
        assert_eq!(pretty().format(&apply_ast), "func(10)");
    }

    #[test]
    fn apply_multiple_arguments() {
        let apply_ast = DatexExpressionData::Apply(Apply {
            base: (DatexExpressionData::VariableAccess(VariableAccess {
                id: 0,
                name: "func".to_string(),
                access_type: ValueAccessType::MoveOrCopy,
            })
            .with_default_span()),
            arguments: vec![
                DatexExpressionData::Integer(10.into()).with_default_span(),
                DatexExpressionData::Text("arg".into()).with_default_span(),
            ],
        })
        .with_default_span();
        assert_eq!(compact().format(&apply_ast), "func(10,\"arg\")");
        assert_eq!(pretty().format(&apply_ast), "func(10,\"arg\")");
    }
}
