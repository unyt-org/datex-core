use crate::ast::error::pattern::Pattern;
use crate::ast::utils::whitespace;
use crate::ast::{DatexExpression, DatexParserTrait};
use crate::compiler::lexer::Token;
use chumsky::prelude::*;

#[derive(Clone, Debug, PartialEq)]
pub enum TupleEntry {
    KeyValue(DatexExpression, DatexExpression),
    Value(DatexExpression),
}

/// A key-value pair inside a tuple
/// Example: (1: "value"), ("key": 123), (("x"+"y"): endpoint(...))
fn tuple_key_value_pair<'a>(
    key: impl DatexParserTrait<'a>,
    expression_without_tuple: impl DatexParserTrait<'a>,
) -> impl DatexParserTrait<'a, TupleEntry> {
    key.then_ignore(just(Token::Colon).padded_by(whitespace()))
        .then(expression_without_tuple)
        .map(|(key, value)| TupleEntry::KeyValue(key, value))
}

/// An entry inside a tuple, either a key-value pair or just a value
fn tuple_entry<'a>(
    key_value_pair: impl DatexParserTrait<'a, TupleEntry>,
    expression_without_tuple: impl DatexParserTrait<'a>,
) -> impl DatexParserTrait<'a, TupleEntry> {
    choice((
        // Key-value pair
        key_value_pair,
        // Just a value with no key
        expression_without_tuple.map(TupleEntry::Value),
    ))
    .boxed()
}

/// A collection of tuple entries with at least two entries, e.g. (1,2)
fn tuple_with_commas<'a>(
    entry: impl DatexParserTrait<'a, TupleEntry>,
) -> impl DatexParserTrait<'a> {
    entry
        .separated_by(just(Token::Comma).padded_by(whitespace()))
        .at_least(2)
        .collect::<Vec<TupleEntry>>()
        .map(DatexExpression::Tuple)
}

/// A single value tuple, e.g. (123,)
fn single_value_tuple<'a>(
    entry: impl DatexParserTrait<'a, TupleEntry>,
) -> impl DatexParserTrait<'a> {
    entry
        .clone()
        .then_ignore(just(Token::Comma))
        .map(|value| vec![value])
        .map(DatexExpression::Tuple)
}

/// A keyed tuple with a single entry, e.g. (key: value)
fn single_keyed_tuple_entry<'a>(
    key_value_pair: impl DatexParserTrait<'a, TupleEntry>,
) -> impl DatexParserTrait<'a> {
    key_value_pair
        .clone()
        .map(|value| vec![value])
        .map(DatexExpression::Tuple)
}

pub fn tuple<'a>(
    key: impl DatexParserTrait<'a>,
    expression_without_tuple: impl DatexParserTrait<'a>,
) -> impl DatexParserTrait<'a> {
    let tuple_key_value_pair =
        tuple_key_value_pair(key, expression_without_tuple.clone());
    let tuple_entry = tuple_entry(
        tuple_key_value_pair.clone(),
        expression_without_tuple.clone(),
    );
    let tuple_with_commas = tuple_with_commas(tuple_entry.clone());
    let single_value_tuple = single_value_tuple(tuple_entry.clone());
    let single_keyed_tuple_entry =
        single_keyed_tuple_entry(tuple_key_value_pair);

    choice((
        tuple_with_commas,
        single_value_tuple,
        single_keyed_tuple_entry,
    ))
    .labelled(Pattern::Custom("tuple"))
    .as_context()
}
