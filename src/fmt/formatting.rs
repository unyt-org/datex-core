use chumsky::span::SimpleSpan;
use pretty::DocAllocator;

use crate::{
    ast::{
        binary_operation::{
            ArithmeticOperator, BinaryOperator, LogicalOperator,
        },
        comparison_operation::ComparisonOperator,
        tree::{
            DatexExpression, DatexExpressionData, List, Map, TypeExpression,
            UnaryOperation, VariableAccess, VariableDeclaration,
        },
        unary_operation::{LogicalUnaryOperator, UnaryOperator},
    },
    compiler::{
        CompileOptions, parse_datex_script_to_rich_ast_simple_error,
        precompiler::RichAst,
    },
    fmt::{
        Assoc, Format, Formatter, Operation, ParentContext,
        options::{BracketStyle, FormattingOptions, VariantFormatting},
    },
    libs::core::CoreLibPointerId,
    values::{
        core_values::integer::{Integer, typed_integer::TypedInteger},
        pointer::PointerAddress,
    },
};

impl<'a> Formatter<'a> {
    /// Formats a typed integer into source code representation based on variant formatting options.
    pub fn typed_integer_to_source_code(
        &'a self,
        ti: &'a TypedInteger,
        span: &'a SimpleSpan,
    ) -> Format<'a> {
        let a = &self.alloc;
        match self.options.variant_formatting {
            VariantFormatting::KeepAll => a.text(self.tokens_at(span)),
            VariantFormatting::WithSuffix => a.text(ti.to_string_with_suffix()),
            VariantFormatting::WithoutSuffix => a.text(ti.to_string()),
        }
    }

    /// Formats a list into source code representation.
    pub fn list_to_source_code(&'a self, elements: &'a List) -> Format<'a> {
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
    pub fn text_to_source_code(&'a self, s: &'a str) -> Format<'a> {
        self.alloc.text(format!("{:?}", s)) // quoted string
    }

    /// Formats a map into source code representation.
    pub fn map_to_source_code(&'a self, map: &'a Map) -> Format<'a> {
        let a = &self.alloc;
        let entries = map.entries.iter().map(|(key, value)| {
            self.format_datex_expression(key)
                + a.text(": ")
                + self.format_datex_expression(value)
        });
        self.wrap_collection(entries, ("{", "}"), ",")
    }
}
