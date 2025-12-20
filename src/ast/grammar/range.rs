use crate::ast::error::error::ParseError;
use crate::ast::grammar::utils::whitespace;
use crate::ast::lexer::Token;
use crate::ast::spanned::Spanned;
use crate::ast::structs::expression::DatexExpression;
use crate::ast::{DatexParserTrait, structs::expression::DatexExpressionData};
use crate::values::core_values::integer::Integer;
use crate::values::core_values::range::Range;
use chumsky::prelude::*;

fn expect_integer(expr: DatexExpression) -> Result<Integer, ParseError> {
    println!("{:?}", expr);
    match expr.data {
        DatexExpressionData::Integer(int) => Ok(int),
        DatexExpressionData::TypedInteger(tint) => Ok(Integer::from(tint)),
        _ => Err(ParseError::new_custom("Expect integer literal".to_string())),
    }
}
pub fn range<'a>(
    inner: impl DatexParserTrait<'a>,
) -> impl DatexParserTrait<'a> {
    inner
        .clone()
        .then(
            just(Token::Range)
                .padded_by(whitespace())
                .ignore_then(inner),
        )
        .map_with(|(start, end), e| {
            let begin = expect_integer(start).unwrap();
            let ending = expect_integer(end).unwrap();
            DatexExpressionData::Range(Range {
                start: begin,
                end: ending,
            })
            .with_span(e.span())
        })
}
