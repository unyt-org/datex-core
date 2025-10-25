use chumsky::span::SimpleSpan;
use pretty::{DocAllocator, DocBuilder, RcAllocator, RcDoc};

use crate::{
    ast::tree::{
        DatexExpression, DatexExpressionData, List, Map, TypeExpression,
        VariableDeclaration,
    },
    values::core_values::integer::{Integer, typed_integer::TypedInteger},
};

type Format<'a> = DocBuilder<'a, RcAllocator, ()>;

pub struct FormattingOptions {
    /// Number of spaces to use for indentation.
    pub indent: usize,

    /// Maximum line width before wrapping occurs.
    pub max_width: usize,

    /// Whether to add trailing commas in collections like lists and maps.
    /// E.g., `[1, 2, 3,]` instead of `[1, 2, 3]`.
    pub trailing_comma: bool,

    /// Whether to add spaces inside collections like lists and maps.
    /// E.g., `[ 1,2,3 ]` instead of `[1,2,3]`.
    pub spaced_collections: bool,

    /// Whether to add spaces inside collections like lists and maps.
    /// E.g., `[1, 2, 3]` instead of `[1,2,3]`.
    pub space_in_collection: bool,

    /// Whether to add spaces around operators.
    /// E.g., `1 + 2` instead of `1+2`.
    pub spaces_around_operators: bool,

    /// Formatting style for type declarations.
    /// Determines how type annotations are spaced and aligned.
    pub type_declaration_formatting: TypeDeclarationFormatting,

    /// Whether to add newlines between statements.
    pub statement_formatting: StatementFormatting,

    /// Formatting style for type variant suffixes.
    pub variant_formatting: VariantFormatting,
}

/// Formatting styles for enum variants.
pub enum VariantFormatting {
    /// Keep the original formatting.
    Keep,
    /// Use variant suffixes.
    WithSuffix,
    /// Do not use variant suffixes.
    WithoutSuffix,
}

/// Formatting styles for statements.
pub enum StatementFormatting {
    NewlineBetween,
    SpaceBetween,
    Compact,
}

/// Formatting styles for type declarations.
pub enum TypeDeclarationFormatting {
    Compact,
    SpaceAroundColon,
    SpaceAfterColon,
}

impl Default for FormattingOptions {
    fn default() -> Self {
        FormattingOptions {
            indent: 4,
            max_width: 40,
            variant_formatting: VariantFormatting::Keep,
            trailing_comma: true,
            spaced_collections: false,
            space_in_collection: true,
            spaces_around_operators: true,
            type_declaration_formatting:
                TypeDeclarationFormatting::SpaceAfterColon,
            statement_formatting: StatementFormatting::NewlineBetween,
        }
    }
}
impl FormattingOptions {
    pub fn compact() -> Self {
        FormattingOptions {
            indent: 2,
            max_width: 40,
            variant_formatting: VariantFormatting::WithoutSuffix,
            trailing_comma: false,
            spaced_collections: false,
            space_in_collection: false,
            spaces_around_operators: false,
            type_declaration_formatting: TypeDeclarationFormatting::Compact,
            statement_formatting: StatementFormatting::Compact,
        }
    }
}
pub struct Formatter {
    options: FormattingOptions,
    alloc: RcAllocator,
}

impl Formatter {
    pub fn new(options: FormattingOptions) -> Self {
        Self {
            options,
            alloc: RcAllocator,
        }
    }

    /// Renders a DatexExpression into a source code string.
    pub fn render(&self, expr: &DatexExpression) -> String {
        self.format_datex_expression(expr)
            .pretty(self.options.max_width)
            .to_string()
    }

    /// Formats a list into source code representation.
    fn list_to_source_code<'a>(&'a self, elements: &'a List) -> Format<'a> {
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
    fn text_to_source_code<'a>(&'a self, s: &'a str) -> Format<'a> {
        self.alloc.text(format!("{:?}", s)) // quoted string
    }

    /// Formats a map into source code representation.
    fn map_to_source_code<'a>(&'a self, map: &'a Map) -> Format<'a> {
        let a = &self.alloc;
        let entries = map.entries.iter().map(|(key, value)| {
            self.format_datex_expression(key)
                + a.text(": ")
                + self.format_datex_expression(value)
        });
        self.wrap_collection(entries, ("{", "}"), ",")
    }

    /// Returns the indentation level
    fn indent(&self) -> isize {
        self.options.indent as isize
    }

    fn typed_integer_to_source_code<'a>(
        &'a self,
        ti: &'a TypedInteger,
        span: &'a SimpleSpan,
    ) -> Format<'a> {
        let a = &self.alloc;
        match self.options.variant_formatting {
            VariantFormatting::Keep => {
                println!("TODO span: {:?}", span);
                todo!("TODO")
            }
            VariantFormatting::WithSuffix => a.text(ti.to_string_with_suffix()),
            VariantFormatting::WithoutSuffix => a.text(ti.to_string()),
        }
    }

    /// Formats a DatexExpression into a DocBuilder for pretty printing.
    fn format_datex_expression<'a>(
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
                todo!("")
            }
            DatexExpressionData::Boolean(b) => a.as_string(b),
            DatexExpressionData::Text(t) => self.text_to_source_code(t),
            DatexExpressionData::Endpoint(e) => a.text(e.to_string()),
            DatexExpressionData::Null => a.text("null"),
            DatexExpressionData::Identifier(l) => a.text(l.clone()),
            DatexExpressionData::Map(map) => self.map_to_source_code(map),
            DatexExpressionData::List(list) => self.list_to_source_code(list),
            DatexExpressionData::CreateRef(expr) => {
                a.text("&") + self.format_datex_expression(expr)
            }
            DatexExpressionData::CreateRefMut(expr) => {
                a.text("&mut ") + self.format_datex_expression(expr)
            }
            DatexExpressionData::CreateRefFinal(expr) => {
                a.text("&final ") + self.format_datex_expression(expr)
            }
            DatexExpressionData::BinaryOperation(op, left, right, _) => {
                let a = &self.alloc;
                (self.format_datex_expression(left)
                    + self.operator_with_spaces(a.text(op.to_string()))
                    + self.format_datex_expression(right))
                .group()
            }
            DatexExpressionData::Statements(statements) => {
                let docs: Vec<_> = statements
                    .statements
                    .iter()
                    .map(|stmt| {
                        self.format_datex_expression(stmt) + a.text(";")
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
            e => panic!("Formatter not implemented for {:?}", e),
        }
    }

    fn format_type_expression<'a>(
        &'a self,
        type_expr: &'a TypeExpression,
    ) -> Format<'a> {
        let a = &self.alloc;
        match type_expr {
            TypeExpression::Integer(ti) => a.text(ti.to_string()),
            TypeExpression::Decimal(td) => a.text(td.to_string()),
            TypeExpression::Boolean(b) => a.text(b.to_string()),
            TypeExpression::Text(t) => a.text(format!("{:?}", t)),
            TypeExpression::Endpoint(ep) => a.text(ep.to_string()),
            TypeExpression::Null => a.text("null"),

            TypeExpression::Ref(inner) => {
                a.text("&") + self.format_type_expression(inner)
            }
            TypeExpression::RefMut(inner) => {
                a.text("&mut") + a.space() + self.format_type_expression(inner)
            }
            TypeExpression::RefFinal(inner) => {
                a.text("&final")
                    + a.space()
                    + self.format_type_expression(inner)
            }

            TypeExpression::Literal(lit) => a.text(lit.to_string()),
            TypeExpression::Variable(_, name) => a.text(name.clone()),

            TypeExpression::GetReference(ptr) => a.text(ptr.to_string()),

            TypeExpression::TypedInteger(typed_integer) => {
                a.text(typed_integer.to_string())
                // TODO: handle variant formatting
            }
            TypeExpression::TypedDecimal(typed_decimal) => {
                a.text(typed_decimal.to_string())
                // TODO: handle variant formatting
            }

            // Lists — `[T, U, V]` or multiline depending on settings
            TypeExpression::StructuralList(elements) => {
                let docs =
                    elements.iter().map(|e| self.format_type_expression(e));
                self.wrap_collection(docs, ("[", "]"), ",")
            }

            TypeExpression::FixedSizeList(_, _) => todo!(),
            TypeExpression::SliceList(_) => todo!(),

            // Intersection: `A & B & C`
            TypeExpression::Intersection(items) => {
                self.wrap_type_collection(items, "&")
            }

            // Union: `A | B | C`
            TypeExpression::Union(items) => {
                self.wrap_type_collection(items, "|")
            }

            TypeExpression::Generic(_, _) => a.text("/* generic TODO */"),

            // Function type: `(x: Int, y: Text) -> Bool`
            TypeExpression::Function {
                parameters,
                return_type,
            } => {
                let params = parameters.iter().map(|(name, ty)| {
                    a.text(name.clone())
                        + self.type_declaration_colon()
                        + self.format_type_expression(ty)
                });
                let params_doc =
                    RcDoc::intersperse(params, a.text(",") + a.space());
                let arrow = self.operator_with_spaces(a.text("->"));
                (a.text("(")
                    + params_doc
                    + a.text(")")
                    + arrow
                    + self.format_type_expression(return_type))
                .group()
            }

            TypeExpression::StructuralMap(items) => {
                let pairs = items.iter().map(|(k, v)| {
                    let key_doc = self.format_type_expression(k);
                    key_doc
                        + self.type_declaration_colon()
                        + self.format_type_expression(v)
                });
                self.wrap_collection(pairs, ("{", "}"), ",")
            }
        }
    }

    fn wrap_type_collection<'a>(
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

    fn type_declaration_colon<'a>(&'a self) -> Format<'a> {
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
    fn operator_with_spaces<'a>(&'a self, text: Format<'a>) -> Format<'a> {
        let a = &self.alloc;
        if self.options.spaces_around_operators {
            a.space() + text + a.space()
        } else {
            text
        }
    }

    /// Wraps a collection of DocBuilders with specified brackets and separator.
    fn wrap_collection<'a>(
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
    use super::*;
    use crate::ast::parse;
    use indoc::indoc;

    #[test]
    fn variant_formatting() {
        let expr = to_expression("42u8");
        assert_eq!(
            to_string(
                &expr,
                FormattingOptions {
                    variant_formatting: VariantFormatting::WithoutSuffix,
                    ..Default::default()
                }
            ),
            "42"
        );
        assert_eq!(
            to_string(
                &expr,
                FormattingOptions {
                    variant_formatting: VariantFormatting::WithSuffix,
                    ..Default::default()
                }
            ),
            "42u8"
        );
        // assert_eq!(
        //     to_string(
        //         &expr,
        //         FormattingOptions {
        //             variant_formatting: VariantFormatting::Keep,
        //             ..Default::default()
        //         }
        //     ),
        //     "42u8"
        // );
    }

    #[test]
    fn statements() {
        let expr = to_expression("1 + 2; var x: integer/u8 = 42; x * 10;");
        assert_eq!(
            to_string(&expr, FormattingOptions::default()),
            indoc! {"
            1 + 2;
            var x: integer/u8 = 42;
            x * 10;"
            }
        );
        assert_eq!(
            to_string(&expr, FormattingOptions::compact()),
            "1+2;var x:integer/u8=42;x*10;"
        );
    }

    #[test]
    fn type_declarations() {
        let expr = to_expression("type(&mut integer/u8)");
        assert_eq!(
            to_string(&expr, FormattingOptions::default()),
            "type(&mut integer/u8)"
        );

        let expr = to_expression("type(text | integer/u16 | decimal/f32)");
        assert_eq!(
            to_string(&expr, FormattingOptions::default()),
            "type(text | integer/u16 | decimal/f32)"
        );
        assert_eq!(
            to_string(&expr, FormattingOptions::compact()),
            "type(text|integer/u16|decimal/f32)"
        );
    }

    #[test]
    fn variable_declaration() {
        let expr = to_expression("var x: &mut integer/u8 = 42;");
        assert_eq!(
            to_string(&expr, FormattingOptions::default()),
            "var x: &mut integer/u8 = 42;"
        );

        assert_eq!(
            to_string(&expr, FormattingOptions::compact()),
            "var x:&mut integer/u8=42;"
        );
    }

    #[test]
    fn binary_operations() {
        let expr = to_expression("1 + 2 * 3 - 4 / 5");
        assert_eq!(
            to_string(&expr, FormattingOptions::default()),
            "1 + 2 * 3 - 4 / 5"
        );
        assert_eq!(to_string(&expr, FormattingOptions::compact()), "1+2*3-4/5");
    }

    #[test]
    fn strings() {
        let expr = to_expression(r#""Hello, \"World\"!""#);
        assert_eq!(
            to_string(&expr, FormattingOptions::default()),
            r#""Hello, \"World\"!""#
        );
    }

    #[test]
    fn lists() {
        // simple list
        let expr = to_expression("[1,2,3,4,5,6,7]");
        assert_eq!(
            to_string(
                &expr,
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
                &expr,
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
                &expr,
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
                &expr,
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

    #[test]
    fn test_format_integer() {
        let expr = to_expression(
            "const x: &mut integer/u8 | text = {a: 1000000, b: [1,2,3,4,5,\"jfdjfsjdfjfsdjfdsjf\", 42, true, {a:1,b:3}], c: 123.456}; x",
        );
        print(&expr, FormattingOptions::default());
        print(&expr, FormattingOptions::compact());

        let expr = to_expression("const x = [1,2,3,4,5,6,7]");
        print(&expr, FormattingOptions::default());
    }

    fn to_expression(s: &str) -> DatexExpression {
        parse(s).unwrap().ast
    }

    fn to_string(expr: &DatexExpression, options: FormattingOptions) -> String {
        let formatter = Formatter::new(options);
        formatter.render(expr)
    }

    fn print(expr: &DatexExpression, options: FormattingOptions) {
        println!("{}", to_string(expr, options));
    }
}
