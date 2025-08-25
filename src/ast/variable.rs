use crate::ast::assignment_operation::{
    AssignmentOperator, assignment_operation,
};
use crate::ast::comparison_operation::comparison_operation;
use crate::ast::error::error::ParseError;
use crate::ast::error::pattern::Pattern;
use crate::ast::utils::whitespace;
use crate::ast::{
    BindingMutability, DatexExpression, DatexParserTrait, ParserRecoverExt,
    ReferenceMutability, VariableKind,
};
use crate::compiler::lexer::Token;
use chumsky::extra::{Err, Full};
use chumsky::prelude::*;

fn internal_variable_declaration(
    name: String,
    value: Box<DatexExpression>,
    type_annotation: Option<DatexExpression>,
    reference_mutability: ReferenceMutability,
    kind: VariableKind,
) -> DatexExpression {
    DatexExpression::VariableDeclaration {
        id: None,
        kind,
        binding_mutability: if kind == VariableKind::Const {
            BindingMutability::Immutable
        } else {
            BindingMutability::Mutable
        },
        reference_mutability,
        name,
        type_annotation: type_annotation.map(Box::new),
        value,
    }
}
fn variable_declaration(
    name: String,
    value: Box<DatexExpression>,
    reference_mutability: ReferenceMutability,
    kind: VariableKind,
) -> DatexExpression {
    internal_variable_declaration(name, value, None, reference_mutability, kind)
}
fn typed_variable_declaration(
    name: String,
    value: Box<DatexExpression>,
    type_annotation: DatexExpression,
    reference_mutability: ReferenceMutability,
    kind: VariableKind,
) -> DatexExpression {
    internal_variable_declaration(
        name,
        value,
        Some(type_annotation),
        reference_mutability,
        kind,
    )
}

pub fn variable_assignment_or_declaration<'a>(
    union: impl DatexParserTrait<'a>,
) -> impl DatexParserTrait<'a> {
    let type_annotation = just(Token::Colon)
        .padded_by(whitespace())
        .ignore_then(union.clone())
        .or_not();
    let assignment_op = assignment_operation();
    let comparison = comparison_operation(union.clone());
    // variable declarations or assignments
    just(Token::Const)
        .or(just(Token::Variable))
        .or_not()
        .padded_by(whitespace())
        .then(select! {
            Token::Identifier(s) => s
        })
        .labelled(Pattern::Declaration)
        .then(type_annotation.clone())
        .then(assignment_op)
        .then(comparison.clone())
        .map(|((((var_keyword, var_name), type_annotation), op), expr)| {
            if let Some(var_type) = var_keyword {
                let (reference_mutability, expr) = match expr {
                    DatexExpression::RefMut(expr) => {
                        (ReferenceMutability::Mutable, expr)
                    }

                    DatexExpression::Ref(expr) => {
                        (ReferenceMutability::Immutable, expr)
                    }

                    expr => (ReferenceMutability::None, Box::new(expr)),
                };
                if op != AssignmentOperator::Assign {
                    return Err(ParseError::new_custom(format!(
                        "Cannot use '{}' operator in variable declaration",
                        op
                    )));
                }
                let var_kind = if var_type == Token::Const {
                    VariableKind::Const
                } else {
                    VariableKind::Var
                };
                Ok(internal_variable_declaration(
                    var_name.to_string(),
                    expr,
                    type_annotation,
                    reference_mutability,
                    var_kind,
                ))
            } else {
                if type_annotation.is_some() {
                    return Err(ParseError::new_custom(
                        "Cannot use type annotation in variable assignment"
                            .into(),
                    ));
                }
                Ok(DatexExpression::AssignmentOperation(
                    op,
                    None,
                    var_name.to_string(),
                    Box::new(expr),
                ))
            }
        })
        .recover_invalid()
}
