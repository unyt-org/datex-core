use std::ops::Range;

use pretty::DocAllocator;

use crate::{
    ast::structs::expression::{
        BinaryOperation, DatexExpression, DatexExpressionData, List, Map,
        VariableAccess, VariableDeclaration,
    },
    fmt::{
        Format, Formatter, Operation, ParentContext,
        options::{StatementFormatting, VariantFormatting},
    },
    values::core_values::{
        decimal::typed_decimal::TypedDecimal,
        integer::typed_integer::TypedInteger,
    },
};
use crate::references::reference::ReferenceMutability;

impl<'a> Formatter<'a> {
    pub fn datex_expression_to_source_code(
        &'a self,
        expr: &'a DatexExpression,
    ) -> Format<'a> {
        let a = &self.alloc;
        match &expr.data {
            DatexExpressionData::Integer(i) => a.as_string(i),
            DatexExpressionData::TypedInteger(ti) => {
                self.typed_integer_to_source_code(ti, &expr.span)
            }
            DatexExpressionData::Decimal(d) => a.as_string(d),
            DatexExpressionData::TypedDecimal(td) => {
                self.typed_decimal_to_source_code(td, &expr.span)
            }
            DatexExpressionData::Boolean(b) => a.as_string(b),
            DatexExpressionData::Text(t) => self.text_to_source_code(t),
            DatexExpressionData::Endpoint(e) => a.text(e.to_string()),
            DatexExpressionData::Null => a.text("null"),
            DatexExpressionData::Identifier(l) => unreachable!(
                "Identifiers should have been resolved before formatting"
            ),
            DatexExpressionData::Map(map) => self.map_to_source_code(map),
            DatexExpressionData::List(list) => self.list_to_source_code(list),
            DatexExpressionData::CreateRef(create_ref) => {
                (match create_ref.mutability {
                    ReferenceMutability::Immutable => a.text("&"),
                    ReferenceMutability::Mutable => a.text("&mut "),
                }) + self.format_datex_expression(&create_ref.expression)
            }
            DatexExpressionData::BinaryOperation(BinaryOperation {
                operator,
                left,
                right,
                ..
            }) => {
                let (precedence, associativity, _is_assoc) =
                    self.binary_operator_info(operator);

                // format children with parent context so they can decide about parens themselves
                let left_doc = self.format_datex_expression_with_parent(
                    left,
                    Some(ParentContext {
                        precedence,
                        associativity,
                        operation: Operation::Binary(operator),
                    }),
                    true,
                );
                let right_doc = self.format_datex_expression_with_parent(
                    right,
                    Some(ParentContext {
                        precedence,
                        associativity,
                        operation: Operation::Binary(operator),
                    }),
                    false,
                );

                let a = &self.alloc;
                (left_doc
                    + self.operator_with_spaces(a.text(operator.to_string()))
                    + right_doc)
                    .group()
            }
            DatexExpressionData::Statements(statements) => {
                let is_terminated = statements.is_terminated;
                let docs: Vec<_> = statements
                    .statements
                    .iter()
                    .enumerate()
                    .map(|(i, stmt)| {
                        self.format_datex_expression(stmt)
                            + (if is_terminated
                                || i < statements.statements.len() - 1
                            {
                                a.text(";")
                            } else {
                                self.alloc.nil()
                            })
                    })
                    .collect();

                let joined = a.intersperse(
                    docs,
                    match self.options.statement_formatting {
                        StatementFormatting::NewlineBetween => a.hardline(),
                        StatementFormatting::SpaceBetween => a.space(),
                        StatementFormatting::Compact => a.nil(),
                    },
                );
                joined.group()
            }
            DatexExpressionData::VariableDeclaration(VariableDeclaration {
                id: _,
                init_expression,
                kind,
                name,
                type_annotation,
            }) => {
                let type_annotation_doc =
                    if let Some(type_annotation) = type_annotation {
                        self.type_declaration_colon()
                            + self.format_type_expression(type_annotation)
                    } else {
                        a.nil()
                    };
                a.text(kind.to_string())
                    + a.space()
                    + a.text(name)
                    + type_annotation_doc
                    + self.operator_with_spaces(a.text("="))
                    + self.format_datex_expression(init_expression)
            }
            DatexExpressionData::Type(type_expr) => {
                let a = &self.alloc;
                let inner = self.format_type_expression(type_expr);
                (a.text("type(") + a.line_() + inner + a.line_() + a.text(")"))
                    .group()
            }
            DatexExpressionData::VariableAccess(VariableAccess {
                name,
                ..
            }) => a.text(name),
            e => panic!("Formatter not implemented for {:?}", e),
        }
    }

    /// Formats a typed integer into source code representation based on variant formatting options.
    fn typed_integer_to_source_code(
        &'a self,
        ti: &'a TypedInteger,
        span: &'a Range<usize>,
    ) -> Format<'a> {
        let a = &self.alloc;
        match self.options.variant_formatting {
            VariantFormatting::KeepAll => a.text(self.tokens_at(span)),
            VariantFormatting::WithSuffix => a.text(ti.to_string_with_suffix()),
            VariantFormatting::WithoutSuffix => a.text(ti.to_string()),
        }
    }

    /// Formats a typed decimal into source code representation based on variant formatting options.
    fn typed_decimal_to_source_code(
        &'a self,
        td: &'a TypedDecimal,
        span: &'a Range<usize>,
    ) -> Format<'a> {
        let a = &self.alloc;
        match self.options.variant_formatting {
            VariantFormatting::KeepAll => a.text(self.tokens_at(span)),
            VariantFormatting::WithSuffix => a.text(td.to_string_with_suffix()),
            VariantFormatting::WithoutSuffix => a.text(td.to_string()),
        }
    }

    /// Formats a list into source code representation.
    fn list_to_source_code(&'a self, elements: &'a List) -> Format<'a> {
        self.wrap_collection(
            elements
                .items
                .iter()
                .map(|e| self.format_datex_expression(e)),
            ("[", "]"),
            ",",
        )
    }

    /// Formats a string into source code representation.
    fn text_to_source_code(&'a self, s: &'a str) -> Format<'a> {
        self.alloc.text(format!("{:?}", s)) // quoted string
    }

    /// Formats a map into source code representation.
    fn map_to_source_code(&'a self, map: &'a Map) -> Format<'a> {
        let a = &self.alloc;
        let entries = map.entries.iter().map(|(key, value)| {
            self.format_datex_expression(key)
                + a.text(":")
                + (self.options.space_in_collection.then(|| a.space()))
                + self.format_datex_expression(value)
        });
        self.wrap_collection(entries, ("{", "}"), ",")
    }
}
