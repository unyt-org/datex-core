use crate::{
    ast::structs::{
        expression::{
            BinaryOperation, ComparisonOperation, DatexExpression,
            DatexExpressionData, UnaryOperation,
        },
    },
    global::operators::{
        BinaryOperator, ComparisonOperator, LogicalUnaryOperator,
        UnaryOperator,
        binary::{ArithmeticOperator, LogicalOperator},
    },
    fmt::{
        Assoc, Format, Formatter, Operation, ParentContext,
        options::BracketStyle,
    },
};

impl<'a> Formatter<'a> {
    /// Handles bracketing of an expression based on the current formatting options.
    pub fn handle_bracketing(
        &'a self,
        expression: &'a DatexExpression,
        doc: Format<'a>,
        parent_ctx: Option<ParentContext<'a>>,
        is_left_child_of_parent: bool,
    ) -> Format<'a> {
        // Handle bracketing based on options
        match self.options.bracket_style {
            BracketStyle::KeepAll => {
                let wraps = expression.wrapped.unwrap_or(0);
                let mut doc = doc;
                for _ in 0..wraps {
                    doc = self.wrap_in_parens(doc);
                }
                doc
            }

            BracketStyle::Minimal => {
                // only wrap if required by precedence
                self.maybe_wrap_by_parent(
                    expression,
                    doc,
                    parent_ctx,
                    is_left_child_of_parent,
                )
            }

            BracketStyle::RemoveDuplicate => {
                // keep at most one original wrap if the user had any, but still don't violate precedence:
                let doc = self.maybe_wrap_by_parent(
                    expression,
                    doc,
                    parent_ctx,
                    is_left_child_of_parent,
                );
                if expression.wrapped.unwrap_or(0) > 0 {
                    // FIXME: this may double-wrap in some cases; a more precise check would be needed
                    self.wrap_in_parens(doc)
                } else {
                    doc
                }
            }
        }
    }

    /// Decides whether to wrap an expression in parentheses based on its parent context.
    pub fn maybe_wrap_by_parent(
        &'a self,
        expression: &'a DatexExpression,
        inner: Format<'a>,
        parent_ctx: Option<ParentContext<'a>>,
        is_left_child_of_parent: bool,
    ) -> Format<'a> {
        // If there's no parent context, nothing forces parentheses.
        if parent_ctx.is_none() {
            return inner;
        }

        let need = self.needs_parens_for_child_expr(
            expression,
            &parent_ctx.unwrap(),
            is_left_child_of_parent,
        );

        if need {
            self.wrap_in_parens(inner)
        } else {
            inner
        }
    }

    /// Returns information about a binary operator: (precedence, associativity, is_associative)
    pub fn binary_operator_info(
        &self,
        op: &BinaryOperator,
    ) -> (u8, Assoc, bool) {
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
            DatexExpressionData::BinaryOperation(BinaryOperation {
                operator,
                ..
            }) => {
                let (prec, _, _) = self.binary_operator_info(operator);
                prec
            }
            DatexExpressionData::ComparisonOperation(ComparisonOperation {
                operator,
                ..
            }) => {
                let (prec, _, _) = self.comparison_operator_info(operator);
                prec
            }
            DatexExpressionData::UnaryOperation(UnaryOperation {
                operator: op,
                ..
            }) => {
                let (prec, _, _) = self.unary_operator_info(op);
                prec
            }
            DatexExpressionData::CreateRef(_) => 40,
            _ => 255, // never need parens
        }
    }

    /// Decide if a child binary expression needs parentheses when placed under a parent operator.
    /// `parent_prec` is precedence of parent operator, `parent_assoc` its associativity.
    /// `is_left_child` indicates whether the child is the left operand.
    fn needs_parens_for_child_expr(
        &self,
        child: &DatexExpression,
        parent_context: &ParentContext<'a>,
        is_left_child: bool,
    ) -> bool {
        // compute child's precedence (based on its expression kind)
        let child_prec = self.expression_precedence(child);

        if child_prec < parent_context.precedence {
            return true; // child binds weaker -> parens required
        }
        if child_prec > parent_context.precedence {
            return false; // child binds stronger -> safe without parens
        }

        // equal precedence, need to inspect operator identity & associativity
        // If both child and parent are binary/comparison/unary, we can check operator identity
        // and whether that operator is associative (so we can drop parens for same-op associative cases).

        // check if same operator and is associative
        let same_op_and_assoc = match (&child.data, &parent_context.operation) {
            (
                DatexExpressionData::BinaryOperation(BinaryOperation {
                    operator,
                    ..
                }),
                Operation::Binary(parent_op),
            ) => {
                let (_, _, c_is_assoc) = self.binary_operator_info(operator);
                operator == *parent_op && c_is_assoc
            }
            (
                DatexExpressionData::ComparisonOperation(ComparisonOperation {
                    operator,
                    ..
                }),
                Operation::Comparison(parent_op),
            ) => {
                let (_, _, c_is_assoc) =
                    self.comparison_operator_info(operator);
                operator == *parent_op && c_is_assoc
            }
            (
                DatexExpressionData::UnaryOperation(UnaryOperation {
                    operator,
                    ..
                }),
                Operation::Unary(parent_op),
            ) => {
                let (_, _, c_is_assoc) = self.unary_operator_info(operator);
                operator == *parent_op && c_is_assoc
            }
            _ => false,
        };

        if same_op_and_assoc {
            // associative same op and precedence -> safe without parens
            return false;
        }

        // fallback to parent associativity + which side the child is on
        match parent_context.associativity {
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
}

#[cfg(test)]
mod tests {
    use crate::fmt::options::{
        BracketStyle, FormattingOptions, VariantFormatting,
    };

    use super::*;

    fn to_string(script: &str, options: FormattingOptions) -> String {
        let formatter = Formatter::new(script, options);
        formatter.render()
    }

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
}
