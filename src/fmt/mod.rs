use core::ops::Range;

use crate::ast::expressions::{DatexExpression, VariableAccess};
use crate::ast::type_expressions::{
    CallableTypeExpression, TypeExpression, TypeExpressionData,
    TypeVariantAccess,
};
use crate::parser::ParserOptions;
use crate::{
    compiler::precompiler::precompiled_ast::RichAst,
    compiler::{CompileOptions, parse_datex_script_to_rich_ast_simple_error},
    fmt::options::{FormattingOptions, TypeDeclarationFormatting},
    global::operators::{BinaryOperator, ComparisonOperator, UnaryOperator},
    libs::core::CoreLibPointerId,
};
use pretty::{DocAllocator, DocBuilder, RcAllocator, RcDoc};

mod bracketing;
mod formatting;
pub mod options;

pub type Format<'a> = DocBuilder<'a, RcAllocator, ()>;

pub struct Formatter<'a> {
    ast: RichAst,
    script: &'a str,
    options: FormattingOptions,
    alloc: RcAllocator,
}

#[derive(Debug)]
/// Represents a parent operation for formatting decisions.
pub enum Operation<'a> {
    Binary(&'a BinaryOperator),
    Comparison(&'a ComparisonOperator),
    Unary(&'a UnaryOperator),
    Statements,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Assoc {
    Left,
    Right,
    None,
}

pub struct ParentContext<'a> {
    precedence: u8,
    associativity: Assoc,
    operation: Operation<'a>,
}

impl<'a> Formatter<'a> {
    pub fn new(script: &'a str, options: FormattingOptions) -> Self {
        let ast = parse_datex_script_to_rich_ast_simple_error(
            script,
            &mut CompileOptions {
                // Preserve scoping information for accurate formatting
                parser_options: ParserOptions {
                    preserve_scoping: true,
                },
                ..Default::default()
            },
        )
        .expect("Failed to parse Datex script");
        Self {
            ast,
            script,
            options,
            alloc: RcAllocator,
        }
    }

    fn tokens_at(&self, span: &Range<usize>) -> &'a str {
        &self.script[span.start..span.end]
    }

    pub fn render(&self) -> String {
        self.render_expression(&self.ast.ast)
    }

    /// Renders a DatexExpression into a source code string.
    fn render_expression(&self, expr: &DatexExpression) -> String {
        self.format_datex_expression(expr)
            .pretty(self.options.max_width)
            .to_string()
    }

    /// Returns the indentation level
    fn indent(&self) -> isize {
        self.options.indent as isize
    }

    // Formats a DatexExpression into a DocBuilder for pretty printing.
    fn format_datex_expression(
        &'a self,
        expr: &'a DatexExpression,
    ) -> Format<'a> {
        self.format_datex_expression_with_parent(expr, None, false)
    }

    /// Formats a DatexExpression into a DocBuilder for pretty printing.
    fn format_datex_expression_with_parent(
        &'a self,
        expr: &'a DatexExpression,
        parent_ctx: Option<ParentContext<'a>>,
        is_left_child_of_parent: bool,
    ) -> Format<'a> {
        self.handle_bracketing(
            expr,
            self.datex_expression_to_source_code(expr),
            parent_ctx,
            is_left_child_of_parent,
        )
    }

    /// Wraps a DocBuilder in parentheses with proper line breaks.
    fn wrap_in_parens(&'a self, doc: Format<'a>) -> Format<'a> {
        let a = &self.alloc;
        (a.text("(") + a.line_() + doc + a.line_() + a.text(")")).group()
    }

    /// Formats a TypeExpression into a DocBuilder for pretty printing.
    fn format_type_expression(
        &'a self,
        type_expr: &'a TypeExpression,
    ) -> Format<'a> {
        let a = &self.alloc;
        println!("formatting type expression: {:?}", type_expr);
        match &type_expr.data {
            TypeExpressionData::VariantAccess(TypeVariantAccess {
                name,
                variant,
                ..
            }) => a.text(format!("{}/{}", name, variant)),
            TypeExpressionData::Integer(ti) => a.text(ti.to_string()),
            TypeExpressionData::Decimal(td) => a.text(td.to_string()),
            TypeExpressionData::Boolean(b) => a.text(b.to_string()),
            TypeExpressionData::Text(t) => a.text(format!("{:?}", t)),
            TypeExpressionData::Endpoint(ep) => a.text(ep.to_string()),
            TypeExpressionData::Null => a.text("null"),
            TypeExpressionData::Unit => a.text("()"),

            TypeExpressionData::Ref(inner) => {
                a.text("&") + self.format_type_expression(inner)
            }
            TypeExpressionData::RefMut(inner) => {
                a.text("&mut") + a.space() + self.format_type_expression(inner)
            }
            TypeExpressionData::Identifier(lit) => a.text(lit.to_string()),
            TypeExpressionData::VariableAccess(VariableAccess {
                name, ..
            }) => a.text(name.clone()),

            TypeExpressionData::GetReference(ptr) => {
                if let Ok(core_lib) = CoreLibPointerId::try_from(ptr) {
                    a.text(core_lib.to_string())
                } else {
                    a.text(ptr.to_string())
                }
            }

            TypeExpressionData::TypedInteger(typed_integer) => {
                a.text(typed_integer.to_string())
                // TODO #625: handle variant formatting
            }
            TypeExpressionData::TypedDecimal(typed_decimal) => {
                a.text(typed_decimal.to_string())
                // TODO #626: handle variant formatting
            }

            // Lists — `[T, U, V]` or multiline depending on settings
            TypeExpressionData::StructuralList(elements) => {
                let docs =
                    elements.0.iter().map(|e| self.format_type_expression(e));
                self.wrap_collection(docs, ("[", "]"), ",")
            }

            TypeExpressionData::FixedSizeList(list) => {
                core::todo!("#627 Undescribed by author.")
            }
            TypeExpressionData::SliceList(_) => {
                core::todo!("#628 Undescribed by author.")
            }

            // Intersection: `A & B & C`
            TypeExpressionData::Intersection(items) => {
                self.wrap_type_collection(&items.0, "&")
            }

            // Union: `A | B | C`
            TypeExpressionData::Union(items) => {
                self.wrap_type_collection(&items.0, "|")
            }

            TypeExpressionData::GenericAccess(access) => {
                core::todo!("#629 Undescribed by author.")
            }

            // Callable type, e.g. `function (x: integer, y: text) -> boolean`
            TypeExpressionData::Callable(CallableTypeExpression {
                kind,
                parameter_types,
                rest_parameter_type,
                return_type,
                yeet_type,
            }) => {
                // TODO #630: handle full signature
                let params = parameter_types.iter().map(|(name, ty)| {
                    a.text(name.clone().unwrap_or_else(|| "_".to_string()))
                        + self.type_declaration_colon()
                        + self.format_type_expression(ty)
                });
                let params_doc =
                    RcDoc::intersperse(params, a.text(",") + a.space());
                let arrow = self.operator_with_spaces(a.text("->"));
                todo!("#631 Undescribed by author.")
            }

            TypeExpressionData::StructuralMap(items) => {
                let pairs = items.0.iter().map(|(k, v)| {
                    let key_doc = self.format_type_expression(k);
                    key_doc
                        + self.type_declaration_colon()
                        + self.format_type_expression(v)
                });
                self.wrap_collection(pairs, ("{", "}"), ",")
            }

            TypeExpressionData::Recover => a.text("/*recover*/"),
        }
    }

    /// Wraps a collection of type expressions with a specified operator.
    fn wrap_type_collection(
        &'a self,
        list: &'a [TypeExpression],
        op: &'a str,
    ) -> Format<'a> {
        let a = &self.alloc;

        // Operator doc with configurable spacing or line breaks
        let op_doc = if self.options.spaces_around_operators {
            a.softline() + a.text(op) + a.softline()
        } else {
            a.text(op)
        };

        // Format all type expressions
        let docs = list.iter().map(|expr| self.format_type_expression(expr));

        // Combine elements with operator between
        a.nil().append(
            RcDoc::intersperse(docs, op_doc).group().nest(self.indent()),
        )
    }

    /// Returns a DocBuilder for the colon in type declarations based on formatting options.
    fn type_declaration_colon(&'a self) -> Format<'a> {
        let a = &self.alloc;
        match self.options.type_declaration_formatting {
            TypeDeclarationFormatting::Compact => a.text(":"),
            TypeDeclarationFormatting::SpaceAroundColon => {
                a.space() + a.text(":") + a.space()
            }
            TypeDeclarationFormatting::SpaceAfterColon => {
                a.text(":") + a.space()
            }
        }
    }

    /// Returns an operator DocBuilder with optional spaces around it.
    fn operator_with_spaces(&'a self, text: Format<'a>) -> Format<'a> {
        let a = &self.alloc;
        if self.options.spaces_around_operators {
            a.space() + text + a.space()
        } else {
            text
        }
    }

    /// Wraps a collection of DocBuilders with specified brackets and separator.
    fn wrap_collection(
        &'a self,
        list: impl Iterator<Item = DocBuilder<'a, RcAllocator, ()>> + 'a,
        brackets: (&'a str, &'a str),
        sep: &'a str,
    ) -> DocBuilder<'a, RcAllocator, ()> {
        let a = &self.alloc;
        let sep_doc = a.text(sep);

        // Optional spacing inside brackets
        let padding = if self.options.spaced_collections {
            a.line()
        } else {
            a.line_()
        };

        // Build joined elements
        let separator = if self.options.space_in_collection {
            sep_doc + a.line()
        } else {
            sep_doc + a.line_()
        };

        let joined = RcDoc::intersperse(list, separator).append(
            if self.options.trailing_comma {
                a.text(sep)
            } else {
                a.nil()
            },
        );

        a.text(brackets.0)
            .append((padding.clone() + joined).nest(self.indent()))
            .append(padding)
            .append(a.text(brackets.1))
            .group()
    }
}

#[cfg(test)]
mod tests {
    use crate::fmt::options::VariantFormatting;

    use super::*;
    use crate::parser::Parser;
    use indoc::indoc;

    #[test]
    fn ensure_unchanged() {
        let script = "const x = {a: 1000000, b: [1,2,3,4,5,\"jfdjfsjdfjfsdjfdsjf\", 42, true, {a:1,b:3}], c: 123.456}; x";
        let ast_original = Parser::parse_with_default_options(script).unwrap();
        let formatted = to_string(script, FormattingOptions::default());
        let ast_new = Parser::parse_with_default_options(&formatted).unwrap();
        assert_eq!(ast_original, ast_new);
    }

    #[test]
    #[ignore]
    fn demo() {
        let expr = "const x: &mut integer/u8 | text = {a: 1000000, b: [1,2,3,4,5,\"jfdjfsjdfjfsdjfdsjf\", 42, true, {a:1,b:3}], c: 123.456}; x";
        print(expr, FormattingOptions::default());
        print(expr, FormattingOptions::compact());

        let expr = "const x = [1,2,3,4,5,6,7]";
        print(expr, FormattingOptions::default());
    }

    #[test]
    fn variant_formatting() {
        let expr = "42u8";
        assert_eq!(
            to_string(
                expr,
                FormattingOptions {
                    variant_formatting: VariantFormatting::WithoutSuffix,
                    ..Default::default()
                }
            ),
            "42"
        );
        assert_eq!(
            to_string(
                expr,
                FormattingOptions {
                    variant_formatting: VariantFormatting::WithSuffix,
                    ..Default::default()
                }
            ),
            "42u8"
        );
        assert_eq!(
            to_string(
                expr,
                FormattingOptions {
                    variant_formatting: VariantFormatting::KeepAll,
                    ..Default::default()
                }
            ),
            "42u8"
        );
    }

    #[test]
    fn statements() {
        let expr = "1 + 2; var x: integer/u8 = 42; x * 10;";
        assert_eq!(
            to_string(expr, FormattingOptions::default()),
            indoc! {"
            1 + 2;
            var x: integer/u8 = 42;
            x * 10;"
            }
        );
        assert_eq!(
            to_string(expr, FormattingOptions::compact()),
            "1+2;var x:integer/u8=42;x*10;"
        );
    }

    #[test]
    fn type_declarations() {
        let expr = "type<&mut integer/u8>";
        assert_eq!(
            to_string(expr, FormattingOptions::default()),
            "type<&mut integer/u8>"
        );

        let expr = "type<text | integer/u16 | decimal/f32>";
        assert_eq!(
            to_string(expr, FormattingOptions::default()),
            "type<text | integer/u16 | decimal/f32>"
        );
        assert_eq!(
            to_string(expr, FormattingOptions::compact()),
            "type<text|integer/u16|decimal/f32>"
        );
    }

    #[test]
    fn variable_declaration() {
        let expr = "var x: &mut integer/u8 = 42;";
        assert_eq!(
            to_string(expr, FormattingOptions::default()),
            "var x: &mut integer/u8 = 42;"
        );

        assert_eq!(
            to_string(expr, FormattingOptions::compact()),
            "var x:&mut integer/u8=42;"
        );
    }

    #[test]
    fn binary_operations() {
        let expr = "1 + 2 * 3 - 4 / 5";
        assert_eq!(
            to_string(expr, FormattingOptions::default()),
            "1 + 2 * 3 - 4 / 5"
        );
        assert_eq!(to_string(expr, FormattingOptions::compact()), "1+2*3-4/5");
    }

    #[test]
    fn text() {
        let expr = r#""Hello, \"World\"!""#;
        assert_eq!(
            to_string(expr, FormattingOptions::default()),
            r#""Hello, \"World\"!""#
        );
    }

    #[test]
    fn lists() {
        // simple list
        let expr = "[1,2,3,4,5,6,7]";
        assert_eq!(
            to_string(
                expr,
                FormattingOptions {
                    max_width: 40,
                    space_in_collection: false,
                    trailing_comma: false,
                    spaced_collections: false,
                    ..Default::default()
                }
            ),
            "[1,2,3,4,5,6,7]"
        );

        // spaced list
        assert_eq!(
            to_string(
                expr,
                FormattingOptions {
                    max_width: 40,
                    space_in_collection: true,
                    trailing_comma: false,
                    spaced_collections: false,
                    ..Default::default()
                }
            ),
            "[1, 2, 3, 4, 5, 6, 7]"
        );

        // spaced list with trailing comma
        assert_eq!(
            to_string(
                expr,
                FormattingOptions {
                    max_width: 40,
                    space_in_collection: true,
                    trailing_comma: true,
                    spaced_collections: true,
                    ..Default::default()
                }
            ),
            "[ 1, 2, 3, 4, 5, 6, 7, ]"
        );

        // wrapped list
        assert_eq!(
            to_string(
                expr,
                FormattingOptions {
                    indent: 4,
                    max_width: 10,
                    space_in_collection: true,
                    trailing_comma: true,
                    spaced_collections: true,
                    ..Default::default()
                }
            ),
            indoc! {"
            [
                1,
                2,
                3,
                4,
                5,
                6,
                7,
            ]"}
        );
    }

    fn to_string(script: &str, options: FormattingOptions) -> String {
        let formatter = Formatter::new(script, options);
        formatter.render()
    }

    fn print(script: &str, options: FormattingOptions) {
        println!("{}", to_string(script, options));
    }
}
