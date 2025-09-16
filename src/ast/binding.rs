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
    BindingMutability, DatexExpression, DatexParserTrait, ParserRecoverExt,
    TypeExpression, VariableKind,
};
use crate::values::reference::ReferenceMutability;
use chumsky::prelude::*;
pub type VariableId = usize;

fn create_variable_declaration(
    name: String,
    value: Box<DatexExpression>,
    type_annotation: Option<TypeExpression>,
    reference_mutability: Option<ReferenceMutability>,
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
        type_annotation,
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
            let (reference_mutability, expr) = match expr {
                DatexExpression::RefMut(expr) => {
                    (Some(ReferenceMutability::Mutable), expr)
                }
                DatexExpression::Ref(expr) => {
                    (Some(ReferenceMutability::Immutable), expr)
                }
                expr => (None, Box::new(expr)),
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
// fn type_declaration<'a>(
//     union: impl DatexParserTrait<'a>,
// ) -> impl DatexParserTrait<'a> {
//     let generic = just(Token::LeftAngle)
//         .ignore_then(union.clone())
//         .then_ignore(just(Token::RightAngle))
//         .or_not();

//     just(Token::Identifier("type".to_string()))
//         .padded_by(whitespace())
//         .ignore_then(select! { Token::Identifier(name) => name })
//         .then(generic)
//         .then_ignore(just(Token::Assign).padded_by(whitespace()))
//         .then(union)
//         .map(|((name, generic), expr)| DatexExpression::TypeDeclaration {
//             id: None,
//             generic: generic.map(Box::new),
//             name: name.to_string(),
//             value: Box::new(expr),
//         })
//         .labelled(Pattern::Declaration)
//         .as_context()
// }

/// A declaration or assignment, e.g. `var x = 42;`, `const x = 69`, `x = 43;`, or `type x = 42`
pub fn declaration_or_assignment<'a>(
    union: impl DatexParserTrait<'a>,
) -> impl DatexParserTrait<'a> {
    choice((
        type_declaration(),
        variable_declaration(union.clone()),
        variable_assignment(union),
    ))
}
