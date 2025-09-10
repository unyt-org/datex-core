use crate::ast::error::pattern::Pattern;
use crate::ast::utils::whitespace;
use crate::ast::{DatexExpression, DatexParserTrait};
use crate::ast::lexer::Token;
use chumsky::prelude::*;


/// A key-value pair inside a map
/// Example: (1: "value"), ("key": 123), (("x"+"y"): endpoint(...))
fn map_key_value_pair<'a>(
    key: impl DatexParserTrait<'a>,
    expression_without_map: impl DatexParserTrait<'a>,
) -> impl DatexParserTrait<'a, (DatexExpression, DatexExpression)> {
    key.then_ignore(just(Token::Colon).padded_by(whitespace()))
        .then(expression_without_map)
}


/// A collection of map entries with at least two entries, e.g. (a:1, b:2)
fn map_with_commas<'a>(
    entry: impl DatexParserTrait<'a, (DatexExpression, DatexExpression)>,
) -> impl DatexParserTrait<'a> {
    entry
        .separated_by(just(Token::Comma).padded_by(whitespace()))
        .at_least(2)
        .collect::<Vec<(DatexExpression, DatexExpression)>>()
        .map(DatexExpression::Map)
}


/// A map with a single entry, e.g. (key: value)
fn single_map_entry<'a>(
    key_value_pair: impl DatexParserTrait<'a, (DatexExpression, DatexExpression)>,
) -> impl DatexParserTrait<'a> {
    key_value_pair
        .clone()
        .map(|value| vec![value])
        .map(DatexExpression::Map)
}

pub fn map<'a>(
    key: impl DatexParserTrait<'a>,
    expression_without_map_and_list: impl DatexParserTrait<'a>,
) -> impl DatexParserTrait<'a> {
    let map_key_value_pair =
        map_key_value_pair(key, expression_without_map_and_list.clone());
    let map_with_commas = map_with_commas(map_key_value_pair.clone());
    let single_map_entry =
        single_map_entry(map_key_value_pair);

    choice((
        map_with_commas,
        single_map_entry,
    ))
    .labelled(Pattern::Custom("map"))
    .as_context()
}
