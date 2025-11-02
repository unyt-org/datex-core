use crate::ast::error::pattern::Pattern;
use crate::ast::grammar::utils::whitespace;
use crate::ast::lexer::Token;
use crate::ast::spanned::Spanned;
use crate::ast::structs::expression::{ApplyChain, List, Map};
use crate::ast::structs::apply_operation::ApplyOperation;
use crate::ast::{DatexExpressionData, DatexParserTrait};
use chumsky::prelude::*;

pub fn chain_without_whitespace_apply<'a>(
    unary: impl DatexParserTrait<'a>,
    key: impl DatexParserTrait<'a>,
    any: impl DatexParserTrait<'a>,
) -> impl DatexParserTrait<'a> {
    unary
        .clone()
        .then(
            choice((
                // generic access: a<b>
                just(Token::LeftAngle)
                    .ignore_then(any.clone())
                    .then_ignore(just(Token::RightAngle))
                    .map(ApplyOperation::GenericAccess),
                // property access
                just(Token::Dot)
                    .padded_by(whitespace())
                    .ignore_then(key)
                    .map(ApplyOperation::PropertyAccess),
            ))
            .repeated()
            .collect::<Vec<_>>(),
        )
        .labelled(Pattern::Custom("chain_no_whitespace_atom"))
        .map_with(|(val, args), e| {
            if args.is_empty() {
                val
            } else {
                DatexExpressionData::ApplyChain(ApplyChain {
                    base: Box::new(val),
                    operations: args,
                })
                .with_span(e.span())
            }
        })
}

pub fn keyed_parameters<'a>(
    key: impl DatexParserTrait<'a>,
    expression: impl DatexParserTrait<'a>,
) -> impl DatexParserTrait<'a> {
    key.then_ignore(just(Token::Colon).padded_by(whitespace()))
        .then(expression.clone())
        .separated_by(just(Token::Comma).padded_by(whitespace()))
        .at_least(0)
        .allow_trailing()
        .collect()
        .padded_by(whitespace())
        .delimited_by(just(Token::LeftParen), just(Token::RightParen))
        .padded_by(whitespace())
        .map_with(|vec, e| {
            DatexExpressionData::Map(Map { entries: vec }).with_span(e.span())
        })
}

pub fn indexed_parameters<'a>(
    expression: impl DatexParserTrait<'a>,
) -> impl DatexParserTrait<'a> {
    expression
        .clone()
        .separated_by(just(Token::Comma).padded_by(whitespace()))
        .at_least(0)
        .allow_trailing()
        .collect()
        .padded_by(whitespace())
        .delimited_by(just(Token::LeftParen), just(Token::RightParen))
        .padded_by(whitespace())
        .map_with(|vec, e| {
            DatexExpressionData::List(List::new(vec)).with_span(e.span())
        })
}

pub fn chain<'a>(
    unary: impl DatexParserTrait<'a>,
    key: impl DatexParserTrait<'a>,
    atom: impl DatexParserTrait<'a>,
    expression: impl DatexParserTrait<'a>,
) -> impl DatexParserTrait<'a> {
    unary
        .clone()
        .then(
            choice((
                // generic access: a<b>
                just(Token::LeftAngle)
                    .ignore_then(expression.clone())
                    .then_ignore(just(Token::RightAngle))
                    .map(ApplyOperation::GenericAccess),
                // apply #1: function call with multiple arguments
                // x(a: 4, b: 5)
                choice((
                    keyed_parameters(key.clone(), expression.clone()),
                    indexed_parameters(expression.clone()),
                ))
                .map(ApplyOperation::FunctionCall),
                // apply #2: an atomic value (e.g. "text", [1,2,3]) - whitespace or newline required before
                // print "sdf"
                just(Token::Whitespace)
                    .repeated()
                    .at_least(1)
                    .ignore_then(atom.padded_by(whitespace()))
                    .map(ApplyOperation::FunctionCall),
                // property access
                // TODO #357: allow integer index access and ranges in dot access notation
                /*
                whatever.0x10.test -> not allowed
                whatever.10.test -> allowed
                whatever.10u8.test -> disallowed
                whatever.1e10.test -> disallowed
                whatever.-1.test -> ?
                whatever.2..5.test -> later, but allowed
                */
                just(Token::Dot)
                    .padded_by(whitespace())
                    .ignore_then(key)
                    .map(ApplyOperation::PropertyAccess),
            ))
            .repeated()
            .collect::<Vec<_>>(),
        )
        .labelled(Pattern::Custom("chain"))
        .map_with(|(val, args), e| {
            // if only single value, return it directly
            if args.is_empty() {
                val
            } else {
                DatexExpressionData::ApplyChain(ApplyChain {
                    base: Box::new(val),
                    operations: args,
                })
                .with_span(e.span())
            }
        })
}
