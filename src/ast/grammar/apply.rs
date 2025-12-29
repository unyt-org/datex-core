use chumsky::Parser;
use chumsky::prelude::*;
use crate::ast::grammar::utils::whitespace;
use crate::ast::lexer::Token;
use crate::ast::{DatexExpressionData, DatexParserTrait};
use crate::ast::spanned::Spanned;
use crate::ast::structs::expression::{Apply};

pub fn apply<'a>(
    atomic_expression: impl DatexParserTrait<'a>,
    atom: impl DatexParserTrait<'a>,
    expression: impl DatexParserTrait<'a>,
) -> impl DatexParserTrait<'a> {
    atomic_expression
        .clone()
        .then(
            choice((
                // apply #1: function call with multiple arguments
                // x(1,2,3)
                expression
                    .separated_by(just(Token::Comma).padded_by(whitespace()))
                    .at_least(0)
                    .allow_trailing()
                    .collect::<Vec<_>>()
                    .padded_by(whitespace())
                    .delimited_by(just(Token::LeftParen), just(Token::RightParen))
                    .padded_by(whitespace()),
                // apply #2: an atomic value (e.g. "text", [1,2,3]) - whitespace or newline required before
                // print "sdf"
                just(Token::Whitespace)
                    .repeated()
                    .at_least(1)
                    .ignore_then(atom.padded_by(whitespace()))
                    .map(|e| vec![e])
            ))
        )
        .map_with(|(base, args), e| {
            DatexExpressionData::Apply(Apply {
                base: Box::new(base),
                arguments: args,
            })
                .with_span(e.span())
        })
}
