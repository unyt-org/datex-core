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

    /// Remove only redundant or duplicate outer brackets, e.g. `((42))` -> `(42)`.
    RemoveDuplicate,

    /// Remove all unnecessary brackets based purely on operator precedence.
    Minimal,
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
            bracket_style: BracketStyle::Minimal,
        }
    }
}

#[derive(Debug)]
/// Represents a parent operation for formatting decisions.
enum Operation<'a> {
    Binary(&'a BinaryOperator),
    Comparison(&'a ComparisonOperator),
    Unary(&'a UnaryOperator),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Assoc {
    Left,
    Right,
    None,
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

    /// Formats a typed integer into source code representation based on variant formatting options.
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
        expression: &'a DatexExpression,
        inner: Format<'a>,
        parent_ctx: Option<(u8, Assoc, Operation<'a>)>,
        is_left_child_of_parent: bool,
    ) -> Format<'a> {
        // If there's no parent context, nothing forces parentheses.
        if parent_ctx.is_none() {
            return inner;
        }

        let (parent_prec, parent_assoc, parent_op) = parent_ctx.unwrap();

        let need = self.needs_parens_for_child_expr(
            expression,
            parent_prec,
            parent_assoc,
            is_left_child_of_parent,
            &parent_op,
        );

        if need {
            self.wrap_in_parens(inner)
        } else {
            inner
        }
    }

    /// Returns information about a binary operator: (precedence, associativity, is_associative)
    fn binary_operator_info(&self, op: &BinaryOperator) -> (u8, Assoc, bool) {
        match op {
            BinaryOperator::Arithmetic(aop) => match aop {
                ArithmeticOperator::Multiply
                | ArithmeticOperator::Divide
                | ArithmeticOperator::Modulo => (20, Assoc::Left, false),
                ArithmeticOperator::Add => (10, Assoc::Left, true), // + is associative
                ArithmeticOperator::Subtract => (10, Assoc::Left, false), // - is not associative
                ArithmeticOperator::Power => (30, Assoc::Right, false),
                _ => (10, Assoc::Left, false),
            },
            BinaryOperator::Logical(lop) => match lop {
                LogicalOperator::And => (5, Assoc::Left, false),
                LogicalOperator::Or => (4, Assoc::Left, false),
            },
            // fallback
            _ => (1, Assoc::None, false),
        }
    }

    /// Returns information about a comparison operator: (precedence, associativity, is_associative)
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
            | ComparisonOperator::GreaterThanOrEqual => (7, Assoc::None, false),
            _ => (7, Assoc::None, false),
        }
    }

    /// Returns information about a unary operator: (precedence, associativity, is_associative)
    fn unary_operator_info(&self, op: &UnaryOperator) -> (u8, Assoc, bool) {
        match op {
            UnaryOperator::Arithmetic(_) => (35, Assoc::Right, false),
            UnaryOperator::Logical(LogicalUnaryOperator::Not) => {
                (35, Assoc::Right, false)
            }
            UnaryOperator::Reference(_) => (40, Assoc::Right, false),
            UnaryOperator::Bitwise(_) => (35, Assoc::Right, false),
        }
    }

    // precedence of an expression (used for children that are not binary/comparison)
    fn expression_precedence(&self, expression: &DatexExpression) -> u8 {
        match &expression.data {
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
            DatexExpressionData::CreateRef(_)
            | DatexExpressionData::CreateRefMut(_)
            | DatexExpressionData::CreateRefFinal(_) => 40,
            _ => 255, // never need parens
        }
    }

    /// Decide if a child binary expression needs parentheses when placed under a parent operator.
    /// `parent_prec` is precedence of parent operator, `parent_assoc` its associativity.
    /// `is_left_child` indicates whether the child is the left operand.
    fn needs_parens_for_child_expr(
        &self,
        child: &DatexExpression,
        parent_prec: u8,
        parent_assoc: Assoc,
        is_left_child: bool,
        parent_op: &Operation<'_>,
    ) -> bool {
        // compute child's precedence (based on its expression kind)
        let child_prec = self.expression_precedence(child);

        if child_prec < parent_prec {
            return true; // child binds weaker -> parens required
        }
        if child_prec > parent_prec {
            return false; // child binds stronger -> safe without parens
        }

        // equal precedence, need to inspect operator identity & associativity
        // If both child and parent are binary/comparison/unary, we can check operator identity
        // and whether that operator is associative (so we can drop parens for same-op associative cases).

        // check if same operator and is associative
        let same_op_and_assoc = match (&child.data, parent_op) {
            (
                DatexExpressionData::BinaryOperation(child_op, _, _, _),
                Operation::Binary(parent_op),
            ) => {
                let (_, _, c_is_assoc) = self.binary_operator_info(child_op);
                child_op == *parent_op && c_is_assoc
            }
            (
                DatexExpressionData::ComparisonOperation(child_op, _, _),
                Operation::Comparison(parent_op),
            ) => {
                let (_, _, c_is_assoc) =
                    self.comparison_operator_info(child_op);
                child_op == *parent_op && c_is_assoc
            }
            (
                DatexExpressionData::UnaryOperation(UnaryOperation {
                    operator: child_op,
                    ..
                }),
                Operation::Unary(parent_op),
            ) => {
                let (_, _, c_is_assoc) = self.unary_operator_info(child_op);
                child_op == *parent_op && c_is_assoc
            }
            _ => false,
        };

        if same_op_and_assoc {
            // associative same op and precedence -> safe without parens
            return false;
        }

        // fallback to parent associativity + which side the child is on
        match parent_assoc {
            Assoc::Left => {
                // left-assoc: right child with equal precedence needs parens
                !is_left_child
            }
            Assoc::Right => {
                // right-assoc: left child with equal precedence needs parens
                is_left_child
            }
            Assoc::None => {
                // non-associative -> always need parens for equal-precedence children
                true
            }
        }
    }

    // Formats a DatexExpression into a DocBuilder for pretty printing.
    pub fn format_datex_expression(
        &'a self,
        expr: &'a DatexExpression,
    ) -> Format<'a> {
        self.format_datex_expression_with_parent(expr, None, false)
    }

    /// Formats a DatexExpression into a DocBuilder for pretty printing.
    fn format_datex_expression_with_parent(
        &'a self,
        expr: &'a DatexExpression,
        parent_ctx: Option<(u8, Assoc, Operation<'a>)>,
        is_left_child_of_parent: bool,
    ) -> Format<'a> {
        let a = &self.alloc;
        let inner_doc = match &expr.data {
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
                let (prec, assoc, _is_assoc) = self.binary_operator_info(op);

                // format children with parent context so they can decide about parens themselves
                let left_doc = self.format_datex_expression_with_parent(
                    left,
                    Some((prec, assoc, Operation::Binary(op))),
                    true,
                );
                let right_doc = self.format_datex_expression_with_parent(
                    right,
                    Some((prec, assoc, Operation::Binary(op))),
                    false,
                );

                let a = &self.alloc;
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
                let wraps = expr.wrapped.unwrap_or(0);
                let mut doc = inner_doc;
                for _ in 0..wraps {
                    doc = self.wrap_in_parens(doc);
                }
                doc
            }

            BracketStyle::Minimal => {
                // only wrap if required by precedence
                self.maybe_wrap_by_parent(
                    expr,
                    inner_doc,
                    parent_ctx,
                    is_left_child_of_parent,
                )
            }

            BracketStyle::RemoveDuplicate => {
                // keep at most one original wrap if the user had any, but still don't violate precedence:
                let doc = self.maybe_wrap_by_parent(
                    expr,
                    inner_doc,
                    parent_ctx,
                    is_left_child_of_parent,
                );
                if expr.wrapped.unwrap_or(0) > 0 {
                    // FIXME: this may double-wrap in some cases; a more precise check would be needed
                    self.wrap_in_parens(doc)
                } else {
                    doc
                }
            }
        }
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
    use super::*;
    use indoc::indoc;

    #[test]
    fn bracketing() {
        let expr = "((42))";
        assert_eq!(
            to_string(
                expr,
                FormattingOptions {
                    bracket_style: BracketStyle::KeepAll,
                    ..Default::default()
                }
            ),
            "((42))"
        );
        assert_eq!(
            to_string(
                expr,
                FormattingOptions {
                    bracket_style: BracketStyle::RemoveDuplicate,
                    ..Default::default()
                }
            ),
            "(42)"
        );
        assert_eq!(
            to_string(
                expr,
                FormattingOptions {
                    bracket_style: BracketStyle::Minimal,
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
    fn binary_operations_wrapped() {
        // (1 + 2) * 3 requires parentheses around (1 + 2)
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

        // 1 + (2 * 3) doesn't require parentheses
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
    fn associative_operations_no_parens_needed() {
        // (1 + 2) + 3  ->  1 + 2 + 3
        let expr = "(1 + 2) + 3";
        assert_eq!(
            to_string(
                expr,
                FormattingOptions {
                    bracket_style: BracketStyle::Minimal,
                    ..Default::default()
                }
            ),
            "1 + 2 + 3"
        );

        // 1 + (2 + 3)  ->  1 + 2 + 3
        let expr = "1 + (2 + 3)";
        assert_eq!(
            to_string(
                expr,
                FormattingOptions {
                    bracket_style: BracketStyle::Minimal,
                    ..Default::default()
                }
            ),
            "1 + 2 + 3"
        );
    }

    #[test]
    fn non_associative_operations_keep_parens() {
        // 1 - (2 - 3) must keep parentheses
        let expr = "1 - (2 - 3)";
        assert_eq!(
            to_string(
                expr,
                FormattingOptions {
                    bracket_style: BracketStyle::Minimal,
                    ..Default::default()
                }
            ),
            "1 - (2 - 3)"
        );

        // (1 - 2) - 3 may drop parentheses
        let expr = "(1 - 2) - 3";
        assert_eq!(
            to_string(
                expr,
                FormattingOptions {
                    bracket_style: BracketStyle::Minimal,
                    ..Default::default()
                }
            ),
            "1 - 2 - 3"
        );
    }

    #[test]
    fn power_operator_right_associative() {
        // Power is right-associative: 2 ^ (3 ^ 4) -> no parens needed
        let expr = "2 ^ (3 ^ 4)";
        assert_eq!(
            to_string(
                expr,
                FormattingOptions {
                    bracket_style: BracketStyle::Minimal,
                    ..Default::default()
                }
            ),
            "2 ^ 3 ^ 4"
        );

        // (2 ^ 3) ^ 4 -> needs parens to preserve grouping
        let expr = "(2 ^ 3) ^ 4";
        assert_eq!(
            to_string(
                expr,
                FormattingOptions {
                    bracket_style: BracketStyle::Minimal,
                    ..Default::default()
                }
            ),
            "(2 ^ 3) ^ 4"
        );
    }

    #[test]
    fn logical_and_or_precedence() {
        // (a && b) || c -> we don't need parentheses
        let expr = "(true && false) || true";
        assert_eq!(
            to_string(
                expr,
                FormattingOptions {
                    bracket_style: BracketStyle::Minimal,
                    ..Default::default()
                }
            ),
            "true && false || true"
        );

        // a && (b || c) -> parentheses required
        let expr = "true && (false || true)";
        assert_eq!(
            to_string(
                expr,
                FormattingOptions {
                    bracket_style: BracketStyle::Minimal,
                    ..Default::default()
                }
            ),
            "true && (false || true)"
        );
    }

    #[test]
    fn remove_duplicate_brackets() {
        // (((1 + 2))) -> (1 + 2)
        let expr = "(((1 + 2)))";
        assert_eq!(
            to_string(
                expr,
                FormattingOptions {
                    bracket_style: BracketStyle::RemoveDuplicate,
                    ..Default::default()
                }
            ),
            "(1 + 2)"
        );
    }

    #[test]
    fn keep_all_brackets_exactly() {
        // Keep exactly what the user wrote
        let expr = "(((1 + 2)))";
        assert_eq!(
            to_string(
                expr,
                FormattingOptions {
                    bracket_style: BracketStyle::KeepAll,
                    ..Default::default()
                }
            ),
            "(((1 + 2)))"
        );
    }

    #[test]
    fn minimal_vs_keepall_equivalence_for_simple() {
        let expr = "1 + 2 * 3";
        let minimal = to_string(
            expr,
            FormattingOptions {
                bracket_style: BracketStyle::Minimal,
                ..Default::default()
            },
        );
        let keep_all = to_string(
            expr,
            FormattingOptions {
                bracket_style: BracketStyle::KeepAll,
                ..Default::default()
            },
        );
        assert_eq!(minimal, keep_all);
        assert_eq!(minimal, "1 + 2 * 3");
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

    #[test]
    fn test_format_integer() {
        let expr = "const x: &mut integer/u8 | text = {a: 1000000, b: [1,2,3,4,5,\"jfdjfsjdfjfsdjfdsjf\", 42, true, {a:1,b:3}], c: 123.456}; x";
        print(expr, FormattingOptions::default());
        print(expr, FormattingOptions::compact());

        let expr = "const x = [1,2,3,4,5,6,7]";
        print(expr, FormattingOptions::default());
    }

    fn to_string(script: &str, options: FormattingOptions) -> String {
        let formatter = Formatter::new(script, options);
        formatter.render()
    }

    fn print(script: &str, options: FormattingOptions) {
        println!("{}", to_string(script, options));
    }
}
