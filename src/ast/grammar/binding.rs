use crate::ast::error::error::ParseError;
use crate::ast::error::pattern::Pattern;
use crate::ast::grammar::assignment_operation::assignment_operation;
use crate::ast::grammar::r#type::{ty, type_declaration};
use crate::ast::grammar::utils::whitespace;
use crate::ast::lexer::Token;
use crate::ast::spanned::Spanned;
use crate::ast::structs::expression::{
    DerefAssignment, VariableAssignment, VariableKind,
};
use crate::ast::structs::expression::{
    PropertyAssignment, VariableDeclaration,
};
use crate::ast::structs::r#type::TypeExpression;
use crate::ast::{
    DatexExpression, DatexExpressionData, DatexParserTrait, ParserRecoverExt,
};
use crate::global::operators::assignment::AssignmentOperator;
use crate::traits::apply;
use chumsky::prelude::*;

fn create_variable_declaration(
    name: String,
    value: DatexExpression,
    type_annotation: Option<TypeExpression>,
    kind: VariableKind,
) -> DatexExpressionData {
    DatexExpressionData::VariableDeclaration(VariableDeclaration {
        id: None,
        kind,
        name,
        type_annotation,
        init_expression: Box::new(value),
    })
}

/// A variable assignment (e.g. `x = 42` or `y += 1`)
pub fn variable_assignment<'a>(
    expression: impl DatexParserTrait<'a>,
) -> impl DatexParserTrait<'a> {
    let assignment_op = assignment_operation();

    select! { Token::Identifier(name) => name }
        .then(assignment_op)
        .then(expression)
        .map_with(|((var_name, operator), expr), e| {
            DatexExpressionData::VariableAssignment(VariableAssignment {
                id: None,
                operator,
                name: var_name.to_string(),
                expression: Box::new(expr),
            })
            .with_span(e.span())
        })
        .labelled(Pattern::Declaration)
        .as_context()
}

/// A variable assignment (e.g. `x.y.0 = 42` or `y.x += 1`)
pub fn property_assignment<'a>(
    apply_chain: impl DatexParserTrait<'a>,
    expression: impl DatexParserTrait<'a>,
) -> impl DatexParserTrait<'a> {
    let assignment_op = assignment_operation();
    apply_chain
        .then(assignment_op)
        .then(expression)
        .map_with(|((access_expression, operator), expr), e| {
            DatexExpressionData::PropertyAssignment(PropertyAssignment {
                operator,
                access_expression: Box::new(access_expression),
                assigned_expression: Box::new(expr),
            })
            .with_span(e.span())
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
        .map_with(
            |(
                ((deref_count, deref_expression), operator),
                assigned_expression,
            ),
             e| {
                DatexExpressionData::DerefAssignment(DerefAssignment {
                    operator,
                    deref_count,
                    deref_expression: Box::new(deref_expression),
                    assigned_expression: Box::new(assigned_expression),
                })
                .with_span(e.span())
            },
        )
        // FIXME #369 assignment instead of declaration
        .labelled(Pattern::Declaration)
        .as_context()
}

/// A variable declaration (e.g. `var x: u32 = 42` or `const y = "Hello"`)
pub fn variable_declaration<'a>(
    union: impl DatexParserTrait<'a>,
) -> impl DatexParserTrait<'a> {
    let type_annotation = just(Token::Colon)
        .padded_by(whitespace())
        .ignore_then(ty())
        .or_not();

    let assignment_op = assignment_operation();
    let keyword = select! {
        Token::Variable => VariableKind::Var,
        Token::Const    => VariableKind::Const,
    };

    keyword
        .padded_by(whitespace())
        .then(select! { Token::Identifier(s) => s })
        .then(type_annotation)
        .then(assignment_op)
        .then(union.clone())
        .map_with(|((((kind, var_name), annotation), op), expr), e| {
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
            )
            .with_span(e.span()))
        })
        .recover_invalid()
        .labelled(Pattern::Declaration)
        .as_context()
}

/// A declaration or assignment, e.g. `var x = 42;`, `const x = 69`, `x = 43;`, or `type x = 42`
pub fn declaration_or_assignment<'a>(
    // apply_chain: impl DatexParserTrait<'a>,
    expression: impl DatexParserTrait<'a>,
    unary: impl DatexParserTrait<'a>,
) -> impl DatexParserTrait<'a> {
    choice((
        // property_assignment(apply_chain, expression.clone()),
        type_declaration(),
        variable_declaration(expression.clone()),
        deref_assignment(expression.clone(), unary),
        variable_assignment(expression.clone()),
    ))
}
