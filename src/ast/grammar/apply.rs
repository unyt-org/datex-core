use chumsky::Parser;
use chumsky::prelude::*;
use crate::ast::lexer::Token;
use crate::ast::{DatexExpressionData, DatexParserTrait};
use crate::ast::spanned::Spanned;
use crate::ast::structs::expression::{Apply};

pub fn apply<'a>(
    lhs: impl DatexParserTrait<'a>,
    atom: impl DatexParserTrait<'a>,
    expression: impl DatexParserTrait<'a>,
) -> impl DatexParserTrait<'a> {
    lhs
        .clone()
        .then(
            expression
                .separated_by(just(Token::Comma))
                .at_least(0)
                .allow_trailing()
                .collect::<Vec<_>>()
                .delimited_by(just(Token::LeftParen), just(Token::RightParen))
        )
        .map_with(|(base, args), e| {
            DatexExpressionData::Apply(Apply {
                base: Box::new(base),
                arguments: args,
            })
                .with_span(e.span())
        })
}
