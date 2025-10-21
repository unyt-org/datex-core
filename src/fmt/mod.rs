use pretty::{DocAllocator, DocBuilder, RcAllocator, RcDoc};

use crate::ast::tree::{
    DatexExpression, DatexExpressionData, VariableDeclaration,
};

type Format<'a> = DocBuilder<'a, RcAllocator, ()>;

pub struct FormattingOptions {
    /// Number of spaces to use for indentation.
    pub indent: usize,

    /// Maximum line width before wrapping occurs.
    pub max_width: usize,

    /// Whether to add type variant suffixes to typed integers and decimals.
    /// E.g., `42u8` instead of `42`.
    pub add_variant_suffix: bool,

    /// Whether to add trailing commas in collections like lists and maps.
    /// E.g., `[1, 2, 3,]` instead of `[1, 2, 3]`.
    pub trailing_comma: bool,

    /// Whether to add spaces inside collections like lists and maps.
    /// E.g., `[ 1,2,3 ]` instead of `[1,2,3]`.
    pub spaced_collections: bool,

    /// Whether to add spaces inside collections like lists and maps.
    /// E.g., `[1, 2, 3]` instead of `[1,2,3]`.
    pub space_in_collection: bool,
}

impl Default for FormattingOptions {
    fn default() -> Self {
        FormattingOptions {
            indent: 4,
            max_width: 40,
            add_variant_suffix: false,
            trailing_comma: true,
            spaced_collections: false,
            space_in_collection: true,
        }
    }
}
impl FormattingOptions {
    pub fn compact() -> Self {
        FormattingOptions {
            indent: 2,
            max_width: 40,
            add_variant_suffix: false,
            trailing_comma: false,
            spaced_collections: false,
            space_in_collection: false,
        }
    }
}
struct Formatter {
    options: FormattingOptions,
    alloc: RcAllocator,
}

impl Formatter {
    fn new(options: FormattingOptions) -> Self {
        Self {
            options,
            alloc: RcAllocator,
        }
    }

    pub fn list_to_source_code<'a>(
        &'a self,
        elements: &'a [DatexExpression],
    ) -> Format<'a> {
        self.wrap_collection(
            elements.iter().map(|e| self.format_datex_expression(e)),
            ("[", "]"),
            ",",
        )
    }

    fn text_to_source_code<'a>(&'a self, s: &'a str) -> Format<'a> {
        self.alloc.text(format!("{:?}", s)) // quoted string
    }

    fn map_to_source_code<'a>(
        &'a self,
        map: &'a [(DatexExpression, DatexExpression)],
    ) -> Format<'a> {
        let a = &self.alloc;

        let entries = map.iter().map(|(key, value)| {
            self.format_datex_expression(key)
                + a.text(": ")
                + self.format_datex_expression(value)
        });

        self.wrap_collection(entries, ("{", "}"), ",")
    }

    fn indent(&self) -> isize {
        self.options.indent as isize
    }

    pub fn format_datex_expression<'a>(
        &'a self,
        expr: &'a DatexExpression,
    ) -> Format<'a> {
        let a = &self.alloc;

        match &expr.data {
            DatexExpressionData::Integer(i) => a.as_string(i),
            DatexExpressionData::TypedInteger(ti) => {
                if self.options.add_variant_suffix {
                    a.text(ti.to_string_with_suffix())
                } else {
                    a.text(ti.to_string())
                }
            }
            DatexExpressionData::Decimal(d) => a.as_string(d),
            DatexExpressionData::TypedDecimal(td) => {
                if self.options.add_variant_suffix {
                    a.text(td.to_string_with_suffix())
                } else {
                    a.text(td.to_string())
                }
            }
            DatexExpressionData::Boolean(b) => a.as_string(b),
            DatexExpressionData::Text(t) => self.text_to_source_code(t),
            DatexExpressionData::Endpoint(e) => a.text(e.to_string()),
            DatexExpressionData::Null => a.text("null"),
            DatexExpressionData::Identifier(l) => a.text(l.clone()),
            DatexExpressionData::Map(map) => self.map_to_source_code(map),
            DatexExpressionData::List(elements) => {
                self.list_to_source_code(elements)
            }
            DatexExpressionData::CreateRef(expr) => {
                a.text("&") + self.format_datex_expression(expr)
            }
            DatexExpressionData::CreateRefMut(expr) => {
                a.text("&mut ") + self.format_datex_expression(expr)
            }
            DatexExpressionData::CreateRefFinal(expr) => {
                a.text("&final ") + self.format_datex_expression(expr)
            }
            DatexExpressionData::BinaryOperation(op, left, right, _) => (self
                .format_datex_expression(left)
                + a.space()
                + a.text(op.to_string())
                + a.space()
                + self.format_datex_expression(right))
            .group(),
            DatexExpressionData::Statements(statements) => {
                let docs: Vec<_> = statements
                    .statements
                    .iter()
                    .map(|stmt| {
                        self.format_datex_expression(stmt) + a.text(";")
                    })
                    .collect();

                let joined = a.intersperse(docs, a.line_());

                // Return a DocBuilder, not RcDoc
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
                        a.text(": ") + a.text("TODO")
                    } else {
                        a.nil()
                    };
                a.text(kind.to_string())
                    + a.space()
                    + a.text(name)
                    + type_annotation_doc
                    + a.space()
                    + a.text("=")
                    + a.space()
                    + self.format_datex_expression(init_expression)
            }
            e => panic!("Formatter not implemented for {:?}", e),
        }
    }

    pub fn render(&self, expr: &DatexExpression) -> String {
        self.format_datex_expression(expr)
            .pretty(self.options.max_width)
            .to_string()
    }

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
    use indoc::indoc;

    use super::*;
    use crate::{
        ast::{
            assignment_operation::AssignmentOperator, parse, tree::VariableKind,
        },
        values::core_values::decimal::Decimal,
    };

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
