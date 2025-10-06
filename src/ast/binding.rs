use crate::ast::assignment_operation::{
    AssignmentOperator, assignment_operation,
};
use crate::ast::comparison_operation::comparison_operation;
use crate::ast::error::error::ParseError;
use crate::ast::error::pattern::Pattern;
use crate::ast::lexer::Token;
use crate::ast::r#type::{r#type, type_declaration};
use crate::ast::utils::whitespace;
use crate::ast::{
    DatexExpression, DatexParserTrait, ParserRecoverExt, TypeExpression,
    VariableKind,
};
use chumsky::prelude::*;
pub type VariableId = usize;

fn create_variable_declaration(
    name: String,
    value: DatexExpression,
    type_annotation: Option<TypeExpression>,
    kind: VariableKind,
) -> DatexExpression {
    DatexExpression::VariableDeclaration {
        id: None,
        kind,
        name,
        type_annotation,
        init_expression: Box::new(value),
    }
}

/// A variable assignment (e.g. `x = 42` or `y += 1`)
pub fn variable_assignment<'a>(
    expression: impl DatexParserTrait<'a>,
) -> impl DatexParserTrait<'a> {
    let assignment_op = assignment_operation();

    select! { Token::Identifier(name) => name }
        .then(assignment_op)
        .then(expression)
        .map(|((var_name, op), expr)| {
            DatexExpression::VariableAssignment(
                op,
                None,
                var_name.to_string(),
                Box::new(expr),
            )
        })
        .labelled(Pattern::Declaration)
        .as_context()
}
pub fn deref_assignment<'a>(
    expression: impl DatexParserTrait<'a>,
    unary: impl DatexParserTrait<'a>,
) -> impl DatexParserTrait<'a> {
    let assignment_op = assignment_operation();

    just(Token::Star)
        .repeated()
        .at_least(1)
        .count()
        .then(unary)
        .then(assignment_op)
        .then(expression)
        .map(|(((deref_count, deref_expression), operator), assigned_expression)| {
            DatexExpression::DerefAssignment {
                operator,
                deref_count,
                deref_expression: Box::new(deref_expression),
                assigned_expression: Box::new(assigned_expression),
            }
        })
        .labelled(Pattern::Declaration)
        .as_context()
}


/// A variable declaration (e.g. `var x: u32 = 42` or `const y = "Hello"`)
pub fn variable_declaration<'a>(
    union: impl DatexParserTrait<'a>,
) -> impl DatexParserTrait<'a> {
    let type_annotation = just(Token::Colon)
        .padded_by(whitespace())
        .ignore_then(r#type())
        .or_not();

    let assignment_op = assignment_operation();
    let keyword = just(Token::Const)
        .map(|_| VariableKind::Const)
        .or(just(Token::Variable).map(|_| VariableKind::Var));

    keyword
        .padded_by(whitespace())
        .then(select! { Token::Identifier(s) => s })
        .then(type_annotation)
        .then(assignment_op)
        .then(union.clone())
        .map(|((((kind, var_name), annotation), op), expr)| {
            if op != AssignmentOperator::Assign {
                return Err(ParseError::new_custom(format!(
                    "Cannot use '{}' operator in variable declaration",
                    op
                )));
            }

            Ok(create_variable_declaration(
                var_name.to_string(),
                expr,
                annotation,
                kind,
            ))
        })
        .recover_invalid()
        .labelled(Pattern::Declaration)
        .as_context()
}

/// A declaration or assignment, e.g. `var x = 42;`, `const x = 69`, `x = 43;`, or `type x = 42`
pub fn declaration_or_assignment<'a>(
    expression: impl DatexParserTrait<'a>,
    unary: impl DatexParserTrait<'a>,
) -> impl DatexParserTrait<'a> {
    choice((
        type_declaration(),
        variable_declaration(expression.clone()),
        deref_assignment(expression.clone(), unary.clone()),
        variable_assignment(expression),
    ))
}
