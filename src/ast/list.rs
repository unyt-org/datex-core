use crate::ast::utils::whitespace;
use crate::ast::{DatexExpression, DatexParserTrait};
use crate::ast::lexer::Token;
use chumsky::prelude::*;
use crate::ast::error::pattern::Pattern;

/// A single value tuple, e.g. (123,)
fn single_value_list<'a>(
    entry: impl DatexParserTrait<'a, DatexExpression>,
) -> impl DatexParserTrait<'a> {
    entry
        .clone()
        .then_ignore(just(Token::Comma))
        .map(|value| vec![value])
        .map(DatexExpression::List)
}


/// A collection of list entries with at least two entries, e.g. 1,2
fn list_with_commas<'a>(
    entry: impl DatexParserTrait<'a, DatexExpression>,
) -> impl DatexParserTrait<'a> {
    entry
        .separated_by(just(Token::Comma).padded_by(whitespace()))
        .at_least(2)
        .collect::<Vec<DatexExpression>>()
        .map(DatexExpression::List)
}


pub fn list<'a>(
    expression_without_map_and_list: impl DatexParserTrait<'a>,
) -> impl DatexParserTrait<'a> {
    let single_value_list =
        single_value_list(expression_without_map_and_list.clone());
    let list_with_commas = list_with_commas(expression_without_map_and_list.clone());

    choice((
        list_with_commas,
        single_value_list,
    ))
        .labelled(Pattern::Custom("list"))
        .as_context()
}
