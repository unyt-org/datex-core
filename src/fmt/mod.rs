use pretty::{DocAllocator, DocBuilder, RcAllocator, RcDoc};

use crate::ast::tree::{
    DatexExpression, DatexExpressionData, VariableDeclaration,
};

pub struct FormattingOptions {
    pub indent: usize,
    pub max_width: usize,
    pub add_variant_suffix: bool,
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

    pub fn text_to_source_code<'a>(
        &'a self,
        s: &'a str,
    ) -> DocBuilder<'a, RcAllocator, ()> {
        self.alloc.text(format!("{:?}", s)) // quoted string
    }

    pub fn list_to_source_code<'a>(
        &'a self,
        elements: &'a [DatexExpression],
    ) -> DocBuilder<'a, RcAllocator, ()> {
        let a = &self.alloc;

        if elements.is_empty() {
            return a.text("[]");
        }

        let docs: Vec<_> = elements
            .iter()
            .map(|e| self.format_datex_expression(e))
            .collect();
        let joined = RcDoc::intersperse(docs, a.text(",") + a.line());

        (a.text("[")
            + (a.line() + joined).nest(self.ident())
            + a.line()
            + a.text("]"))
        .group()
    }

    fn ident(&self) -> isize {
        self.options.indent as isize
    }

    pub fn map_to_source_code<'a>(
        &'a self,
        map: &'a [(DatexExpression, DatexExpression)],
    ) -> DocBuilder<'a, RcAllocator, ()> {
        let a = &self.alloc;

        if map.is_empty() {
            return a.text("{}");
        }

        let entries: Vec<_> = map
            .iter()
            .map(|(k, v)| {
                self.format_datex_expression(k)
                    + a.text(": ")
                    + self.format_datex_expression(v)
            })
            .collect();

        let joined = RcDoc::intersperse(entries, a.text(",") + a.line());

        (a.text("{")
            + (a.line() + joined).nest(self.ident())
            + a.line()
            + a.text("}"))
        .group()
    }

    pub fn format_datex_expression<'a>(
        &'a self,
        expr: &'a DatexExpression,
    ) -> DocBuilder<'a, RcAllocator, ()> {
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

    fn to_expression(s: &str) -> DatexExpression {
        parse(s).unwrap().ast
    }

    #[test]
    fn test_format_integer() {
        let expr = to_expression(
            "const x: &mut integer/u8 | text = {a: 1000000, b: [1,2,3,4,5,\"jfdjfsjdfjfsdjfdsjf\", 42, true, {a:1,b:3}], c: 123.456}; x",
        );
        let formatter = Formatter::new(FormattingOptions {
            indent: 4,
            max_width: 80,
            add_variant_suffix: false,
        });
        let formatted = formatter.render(&expr);
        println!("{}", formatted);
    }
}
