use crate::ast::DatexExpression;
use crate::ast::utils::whitespace;
use crate::compiler::lexer::Token;
use chumsky::extra::{Err, Full};
use chumsky::prelude::*;

#[derive(Clone, Debug, PartialEq)]
pub enum ApplyOperation {
    /// Apply an array type to an argument
    ArrayType,

    /// Apply a function to an argument
    FunctionCall(DatexExpression),
    /// Apply a property access to an argument
    PropertyAccess(DatexExpression),
}

pub fn chain<'a>(
    unary: impl Parser<'a, &'a [Token], DatexExpression, Err<Cheap>> + Clone + 'a,
    key: impl Parser<'a, &'a [Token], DatexExpression, Full<Cheap, (), ()>>
    + Clone
    + 'a,
    array: impl Parser<'a, &'a [Token], DatexExpression, Err<Cheap>> + Clone + 'a,
    object: impl Parser<'a, &'a [Token], DatexExpression, Err<Cheap>> + Clone + 'a,
    wrapped_expression: impl Parser<
        'a,
        &'a [Token],
        DatexExpression,
        Full<Cheap, (), ()>,
    > + Clone
    + 'a,
    atom: impl Parser<'a, &'a [Token], DatexExpression, Err<Cheap>> + Clone + 'a,
) -> impl Parser<'a, &'a [Token], DatexExpression, Err<Cheap>> + Clone + 'a {
    unary
        .then(
            choice((
                // apply #1: a wrapped expression, array, or object - no whitespace required before
                // x () x [] x {}
                choice((wrapped_expression, array, object))
                    .clone()
                    .padded_by(whitespace())
                    .map(ApplyOperation::FunctionCall),
                // apply #2: an atomic value (e.g. "text") - whitespace or newline required before
                // print "sdf"
                just(Token::Whitespace)
                    .repeated()
                    .at_least(1)
                    .ignore_then(atom.padded_by(whitespace()))
                    .map(ApplyOperation::FunctionCall),
                // property access
                just(Token::Dot)
                    .padded_by(whitespace())
                    .ignore_then(key)
                    .map(ApplyOperation::PropertyAccess),
                just(Token::LeftBracket)
                    .ignore_then(just(Token::RightBracket))
                    .map(|_| ApplyOperation::ArrayType),
            ))
            .repeated()
            .collect::<Vec<_>>(),
        )
        .map(|(val, args)| {
            // if only single value, return it directly
            if args.is_empty() {
                val
            } else {
                DatexExpression::ApplyChain(Box::new(val), args)
            }
        })
}
