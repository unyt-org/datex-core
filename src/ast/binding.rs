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
use chumsky::prelude::*;
pub type VariableId = usize;

fn create_variable_declaration(
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

/// A variable assignment (e.g. `x = 42` or `y += 1`)
pub fn variable_assignment<'a>(
    union: impl DatexParserTrait<'a>,
) -> impl DatexParserTrait<'a> {
    let assignment_op = assignment_operation();
    let comparison = comparison_operation(union.clone());

    select! { Token::Identifier(name) => name }
        .then(assignment_op)
        .then(comparison)
        .map(|((var_name, op), expr)| {
            DatexExpression::AssignmentOperation(
                op,
                None,
                var_name.to_string(),
                Box::new(expr),
            )
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
        .ignore_then(union.clone())
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

            Ok(create_variable_declaration(
                var_name.to_string(),
                expr,
                annotation,
                reference_mutability,
                kind,
            ))
        })
        .recover_invalid()
        .labelled(Pattern::Declaration)
        .as_context()
}

/// A type declaration, e.g. `type MyType = { x: 42, y: "John" };`
fn type_declaration<'a>(
    union: impl DatexParserTrait<'a>,
) -> impl DatexParserTrait<'a> {
    just(Token::Identifier("type".to_string()))
        .padded_by(whitespace())
        .ignore_then(select! { Token::Identifier(name) => name })
        .then_ignore(just(Token::Assign).padded_by(whitespace()))
        .then(union)
        .map(|(name, expr)| DatexExpression::TypeDeclaration {
            id: None,
            name: name.to_string(),
            value: Box::new(expr),
        })
        .labelled(Pattern::Declaration)
        .as_context()
}

/// A declaration or assignment, e.g. `var x = 42;`, `const x = 69`, `x = 43;`, or `type x = 42`
pub fn declaration_or_assignment<'a>(
    union: impl DatexParserTrait<'a>,
) -> impl DatexParserTrait<'a> {
    choice((
        type_declaration(union.clone()),
        variable_declaration(union.clone()),
        variable_assignment(union),
    ))
}
