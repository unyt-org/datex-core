use crate::ast::data::expression::Slot;
use crate::ast::data::spanned::Spanned;
use crate::ast::lexer::Token;
use crate::ast::{DatexExpressionData, DatexParserTrait};
use crate::values::pointer::PointerAddress;
use chumsky::prelude::*;

pub fn literal<'a>() -> impl DatexParserTrait<'a> {
    choice((
        select! { Token::True => DatexExpressionData::Boolean(true) },
        select! { Token::False => DatexExpressionData::Boolean(false) },
        select! { Token::Null => DatexExpressionData::Null },
        // TODO #353: Remove clippy ignore
        select! { Token::NamedSlot(s) => DatexExpressionData::Slot(Slot::Named(s[1..].to_string())) },
        select! { Token::PointerAddress(s) => DatexExpressionData::PointerAddress(PointerAddress::try_from(&s[1..]).unwrap()) },
        select! { Token::Slot(s) => DatexExpressionData::Slot(Slot::Addressed(s[1..].parse::<u32>().unwrap())) },
        select! { Token::Placeholder => DatexExpressionData::Placeholder },
        select! { Token::Identifier(name) => DatexExpressionData::Identifier(name) },
    ))
    .map_with(|data, e| data.with_span(e.span()))
}
