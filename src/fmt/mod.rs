use chumsky::span::SimpleSpan;
use pretty::{DocAllocator, DocBuilder, RcAllocator, RcDoc};

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
    libs::core::CoreLibPointerId,
    values::{
        core_values::integer::{Integer, typed_integer::TypedInteger},
        pointer::PointerAddress,
    },
};

type Format<'a> = DocBuilder<'a, RcAllocator, ()>;

#[derive(Clone, Debug, PartialEq, Eq)]
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

    /// Bracketing style for expressions.
    pub bracket_style: BracketStyle,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BracketStyle {
    /// Keep original bracketing as is.
    KeepAll,

    /// Remove only redundant or duplicate outer brackets, e.g. `((42))` → `(42)`.
    RemoveDuplicate,

    /// Remove all unnecessary brackets based purely on operator precedence.
    Minimal,

    /// Don’t use brackets at all unless absolutely required for syntactic validity.
    None,
}

/// Formatting styles for enum variants.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VariantFormatting {
    /// Keep the original formatting.
    KeepAll,
    /// Use variant suffixes.
    WithSuffix,
    /// Do not use variant suffixes.
    WithoutSuffix,
}

/// Formatting styles for statements.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StatementFormatting {
    /// Add a newline between statements.
    NewlineBetween,
    /// Add a space between statements.
    SpaceBetween,
    /// Compact formatting without extra spaces or newlines.
    Compact,
}

/// Formatting styles for type declarations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TypeDeclarationFormatting {
    /// Compact formatting without extra spaces.
    Compact,
    /// Spaces around the colon in type declarations.
    SpaceAroundColon,
    /// Space after the colon in type declarations.
    SpaceAfterColon,
}

impl Default for FormattingOptions {
    fn default() -> Self {
        FormattingOptions {
            indent: 4,
            max_width: 40,
            variant_formatting: VariantFormatting::KeepAll,
            trailing_comma: true,
            spaced_collections: false,
            space_in_collection: true,
            spaces_around_operators: true,
            type_declaration_formatting:
                TypeDeclarationFormatting::SpaceAfterColon,
            statement_formatting: StatementFormatting::NewlineBetween,
            bracket_style: BracketStyle::Minimal,
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
            bracket_style: BracketStyle::None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Assoc {
    Left,
    Right,
    NoneAssoc,
}

pub struct Formatter<'a> {
    ast: RichAst,
    script: &'a str,
    options: FormattingOptions,
    alloc: RcAllocator,
}

impl<'a> Formatter<'a> {
    pub fn new(script: &'a str, options: FormattingOptions) -> Self {
        let ast = parse_datex_script_to_rich_ast_simple_error(
            script,
            &mut CompileOptions::default(),
        )
        .expect("Failed to parse Datex script");
        Self {
            ast,
            script,
            options,
            alloc: RcAllocator,
        }
    }

    fn tokens_at(&self, span: &SimpleSpan) -> &'a str {
        &self.script[span.start..span.end]
    }

    pub fn render(&self) -> String {
        if let Some(ast) = &self.ast.ast {
            self.render_expression(&ast)
        } else {
            "".to_string()
        }
    }

    /// Renders a DatexExpression into a source code string.
    fn render_expression(&self, expr: &DatexExpression) -> String {
        self.format_datex_expression(expr)
            .pretty(self.options.max_width)
            .to_string()
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
                + a.text(": ")
                + self.format_datex_expression(value)
        });
        self.wrap_collection(entries, ("{", "}"), ",")
    }

    /// Returns the indentation level
    fn indent(&self) -> isize {
        self.options.indent as isize
    }

    fn typed_integer_to_source_code(
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

    fn maybe_wrap_by_parent(
        &'a self,
        expr: &'a DatexExpression,
        inner: Format<'a>,
        parent_op: Option<(u8, Assoc, &'a BinaryOperator)>,
        is_left_child_of_parent: bool,
    ) -> Format<'a> {
        // no parent -> nothing to force
        if parent_op.is_none() {
            return inner;
        }
        let (parent_prec, parent_assoc, parent_op_enum) = parent_op.unwrap();

        // If child is a binary op we need to inspect its operator
        match &expr.data {
            DatexExpressionData::BinaryOperation(
                child_op,
                _left,
                _right,
                _,
            ) => {
                // If KeepAll would have kept original wraps - but here we're working Minimal/RemoveDuplicate
                let need = self.needs_parens_for_binary_child(
                    child_op,
                    parent_prec,
                    parent_assoc,
                    is_left_child_of_parent,
                    parent_op_enum,
                );
                if need {
                    self.wrap_in_parens(inner)
                } else {
                    inner
                }
            }
            // If child is non-binary but still had original parentheses and some contexts require them:
            _ => {
                // usually atoms/primary expressions don't need parens
                // but still respect cases where expr.wrapped is > 0 and parent is something that would require.
                // conservative choice: if parent precedence is > child precedence -> need parens
                let child_prec = self.expr_precedence(expr);
                if child_prec < parent_prec {
                    self.wrap_in_parens(inner)
                } else {
                    inner
                }
            }
        }
    }

    fn binary_operator_info(&self, op: &BinaryOperator) -> (u8, Assoc, bool) {
        match op {
            BinaryOperator::Arithmetic(op) => match op {
                &ArithmeticOperator::Multiply | &ArithmeticOperator::Divide => {
                    (20, Assoc::Left, false)
                }
                ArithmeticOperator::Add | ArithmeticOperator::Subtract => {
                    (10, Assoc::Left, false)
                }
                ArithmeticOperator::Power => (30, Assoc::Right, false),
                _ => unimplemented!(),
            },
            BinaryOperator::Logical(op) => match op {
                LogicalOperator::And | LogicalOperator::Or => {
                    (5, Assoc::Left, false)
                }
            },
            _ => unimplemented!(),
        }
    }
    fn comparison_operator_info(
        &self,
        op: &ComparisonOperator,
    ) -> (u8, Assoc, bool) {
        match op {
            ComparisonOperator::Equal
            | ComparisonOperator::NotEqual
            | ComparisonOperator::LessThan
            | ComparisonOperator::LessThanOrEqual
            | ComparisonOperator::GreaterThan
            | ComparisonOperator::GreaterThanOrEqual => {
                (7, Assoc::NoneAssoc, false)
            }
            _ => (1, Assoc::NoneAssoc, false),
        }
    }
    fn unary_operator_info(&self, op: &UnaryOperator) -> (u8, Assoc, bool) {
        match op {
            UnaryOperator::Arithmetic(op) => match op {
                _ => unimplemented!(),
            },
            UnaryOperator::Logical(op) => match op {
                LogicalUnaryOperator::Not => (35, Assoc::Right, false),
            },
            UnaryOperator::Reference(op) => match op {
                _ => unimplemented!(),
            },
            UnaryOperator::Bitwise(op) => match op {
                _ => unimplemented!(),
            },
        }
    }

    /// precedence of an expression (used when child is not a binary op).
    /// For atoms (identifiers, literals) return very large so they never need parentheses.
    fn expr_precedence(&self, expr: &DatexExpression) -> u8 {
        match &expr.data {
            DatexExpressionData::BinaryOperation(op, _, _, _) => {
                let (prec, _, _) = self.binary_operator_info(op);
                prec
            }
            DatexExpressionData::ComparisonOperation(op, _, _) => {
                let (prec, _, _) = self.comparison_operator_info(op);
                prec
            }
            DatexExpressionData::UnaryOperation(UnaryOperation {
                operator: op,
                ..
            }) => {
                let (prec, _, _) = self.unary_operator_info(op);
                prec
            }
            // unary/prefix: give them higher precedence than binary
            DatexExpressionData::CreateRef(_)
            | DatexExpressionData::CreateRefMut(_)
            | DatexExpressionData::CreateRefFinal(_) => 40,
            // atomic
            _ => 255,
        }
    }

    /// Decide if a child binary expression needs parentheses when placed under a parent operator.
    /// `parent_prec` is precedence of parent operator, `parent_assoc` its associativity.
    /// `is_left_child` indicates whether the child is the left operand.
    fn needs_parens_for_binary_child(
        &self,
        child_op: &BinaryOperator,
        parent_prec: u8,
        parent_assoc: Assoc,
        is_left_child: bool,
        parent_op: &BinaryOperator,
    ) -> bool {
        let (child_prec, _, _) = self.binary_operator_info(child_op);

        if child_prec < parent_prec {
            return true; // child binds weaker -> needs parens
        }
        if child_prec > parent_prec {
            return false; // child binds tighter -> safe
        }

        // equal precedence: associativity & position decide
        if parent_assoc == Assoc::Left {
            // left-assoc: the right child with same precedence needs parens
            !is_left_child
        } else if parent_assoc == Assoc::Right {
            // right-assoc: the left child with same precedence needs parens
            is_left_child
        } else {
            // non-assoc -> always need parens if precedence equal
            true
        }
    }

    // Example of the small public wrapper you already have:
    pub fn format_datex_expression(
        &'a self,
        expr: &'a DatexExpression,
    ) -> Format<'a> {
        // top-level: no parent context
        self.format_datex_expression_with_parent(expr, None, false)
    }

    /// Formats a DatexExpression into a DocBuilder for pretty printing.
    fn format_datex_expression_with_parent(
        &'a self,
        expr: &'a DatexExpression,
        parent_op: Option<(u8, Assoc, &'a BinaryOperator)>,
        is_left_child_of_parent: bool,
    ) -> Format<'a> {
        let a = &self.alloc;

        let mut inner_doc = match &expr.data {
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
            // DatexExpressionData::BinaryOperation(op, left, right, _) => {
            //     let a = &self.alloc;
            //     (self.format_datex_expression(left)
            //         + self.operator_with_spaces(a.text(op.to_string()))
            //         + self.format_datex_expression(right))
            //     .group()
            // }
            DatexExpressionData::BinaryOperation(op, left, right, _) => {
                let (prec, assoc, _assoc_flag) = self.binary_operator_info(op);
                // format children with this op as parent context
                let left_doc = self.format_datex_expression_with_parent(
                    left,
                    Some((prec, assoc, op)),
                    true,
                );
                let right_doc = self.format_datex_expression_with_parent(
                    right,
                    Some((prec, assoc, op)),
                    false,
                );

                // combine with operator doc
                (left_doc
                    + self.operator_with_spaces(a.text(op.to_string()))
                    + right_doc)
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
            DatexExpressionData::VariableAccess(VariableAccess {
                name,
                ..
            }) => a.text(name),
            e => panic!("Formatter not implemented for {:?}", e),
        };
        // Handle bracketing based on options
        match self.options.bracket_style {
            BracketStyle::KeepAll => {
                // re-insert the exact # of wraps recorded by the parser
                let wraps = expr.wrapped.unwrap_or(0);
                let mut doc = inner_doc;
                for _ in 0..wraps {
                    doc = self.wrap_in_parens(doc);
                }
                doc
            }

            BracketStyle::None => {
                // never use parentheses; returns possibly incorrect code if they were needed
                inner_doc
            }

            BracketStyle::RemoveDuplicate => {
                // If the parser saw wrapping > 0, keep *one* wrap unless precedence rules force more/less.
                let original_wraps = expr.wrapped.unwrap_or(0);
                if original_wraps == 0 {
                    // no original brackets and we're not force-inserting
                    // but we still must respect precedence rules *if* the parent requires them.
                    self.maybe_wrap_by_parent(
                        expr,
                        inner_doc,
                        parent_op,
                        is_left_child_of_parent,
                    )
                } else {
                    // parser had brackets — keep at most one pair if not required to remove
                    let maybe_wrapped = self.maybe_wrap_by_parent(
                        expr,
                        inner_doc.clone(),
                        parent_op,
                        is_left_child_of_parent,
                    );
                    // If maybe_wrap_by_parent decided NOT to wrap and the user wanted RemoveDuplicate,
                    // we can still decide to keep a single pair if you want (policy choice).
                    // Here: keep one pair only if parentheses are required OR originally present
                    // but do NOT keep multiple duplicate pairs.
                    // We'll choose: keep one if original present OR required by precedence.
                    let kept = match &maybe_wrapped {
                        doc if expr.wrapped.unwrap_or(0) > 0 => {
                            // wrap once (ensure single)
                            self.wrap_in_parens(inner_doc)
                        }
                        _ => maybe_wrapped,
                    };
                    kept
                }
            }

            BracketStyle::Minimal => {
                // Remove parens unless required by operator precedence/associativity
                self.maybe_wrap_by_parent(
                    expr,
                    inner_doc,
                    parent_op,
                    is_left_child_of_parent,
                )
            }
        }
    }

    fn wrap_in_parens(&'a self, doc: Format<'a>) -> Format<'a> {
        let a = &self.alloc;
        (a.text("(") + a.line_() + doc + a.line_() + a.text(")")).group()
    }

    fn format_type_expression(
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

            TypeExpression::GetReference(ptr) => {
                if let Ok(core_lib) = CoreLibPointerId::try_from(ptr) {
                    a.text(core_lib.to_string())
                } else {
                    a.text(ptr.to_string())
                }
            }

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
    use super::*;
    use crate::compiler::{
        CompileOptions, parse_datex_script_to_rich_ast_simple_error,
        precompiler::RichAst,
    };
    use indoc::indoc;

    #[test]
    fn bracketing() {
        let expr = "((42))";
        assert_eq!(
            to_string(
                &expr,
                FormattingOptions {
                    bracket_style: BracketStyle::KeepAll,
                    ..Default::default()
                }
            ),
            "((42))"
        );
        assert_eq!(
            to_string(
                &expr,
                FormattingOptions {
                    bracket_style: BracketStyle::RemoveDuplicate,
                    ..Default::default()
                }
            ),
            "(42)"
        );
        assert_eq!(
            to_string(
                &expr,
                FormattingOptions {
                    bracket_style: BracketStyle::None,
                    ..Default::default()
                }
            ),
            "42"
        );
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
        let expr = "type(&mut integer/u8)";
        assert_eq!(
            to_string(expr, FormattingOptions::default()),
            "type(&mut integer/u8)"
        );

        let expr = "type(text | integer/u16 | decimal/f32)";
        assert_eq!(
            to_string(expr, FormattingOptions::default()),
            "type(text | integer/u16 | decimal/f32)"
        );
        assert_eq!(
            to_string(expr, FormattingOptions::compact()),
            "type(text|integer/u16|decimal/f32)"
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
    #[ignore = "WIP"]
    fn binary_operations_wrapped() {
        let expr = "(1 + 2) * 3 - 4 / 5";
        assert_eq!(
            to_string(
                expr,
                FormattingOptions {
                    bracket_style: BracketStyle::Minimal,
                    ..Default::default()
                }
            ),
            "(1 + 2) * 3 - 4 / 5"
        );

        let expr = "1 + (2 * 3) - 4 / 5";
        assert_eq!(
            to_string(
                expr,
                FormattingOptions {
                    bracket_style: BracketStyle::Minimal,
                    ..Default::default()
                }
            ),
            "1 + 2 * 3 - 4 / 5"
        );
    }

    #[test]
    fn strings() {
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

    #[test]
    fn test_format_integer() {
        let expr = "const x: &mut integer/u8 | text = {a: 1000000, b: [1,2,3,4,5,\"jfdjfsjdfjfsdjfdsjf\", 42, true, {a:1,b:3}], c: 123.456}; x";
        print(expr, FormattingOptions::default());
        print(expr, FormattingOptions::compact());

        let expr = "const x = [1,2,3,4,5,6,7]";
        print(expr, FormattingOptions::default());
    }

    // fn to_expression(s: &str) -> DatexExpression {
    //     parse(s).unwrap().ast
    // }

    // fn to_string(expr: &DatexExpression, options: FormattingOptions) -> String {
    //     let formatter = Formatter::new(options);
    //     formatter.render(expr)
    // }
    fn to_string(script: &str, options: FormattingOptions) -> String {
        let formatter = Formatter::new(script, options);
        formatter.render()
    }

    fn print(script: &str, options: FormattingOptions) {
        println!("{}", to_string(script, options));
    }
}
