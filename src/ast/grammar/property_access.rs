use crate::ast::lexer::Token;
use crate::ast::structs::expression::{ PropertyAccess};
use crate::ast::{DatexExpressionData, DatexParserTrait};
use chumsky::prelude::*;
use datex_core::ast::spanned::Spanned;

/// A property access chain, e.g. `a.b.(c).1
pub fn property_access<'a>(
    lhs: impl DatexParserTrait<'a>,
    atomic_expression: impl DatexParserTrait<'a>,
) -> impl DatexParserTrait<'a> {
    lhs
        .clone()
        .then_ignore(just(Token::Dot))
        .then(
            atomic_expression.clone()
        )
        .map_with(|(base_expr, property_expr), e| {
            DatexExpressionData::PropertyAccess(
                PropertyAccess {
                    base: Box::new(base_expr),
                    property: Box::new(property_expr),
                },
            ).with_span(e.span())
        })
}