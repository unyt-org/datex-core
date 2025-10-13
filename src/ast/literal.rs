use crate::ast::lexer::Token;
use crate::ast::{DatexExpressionData, DatexParserTrait};
use crate::values::pointer::PointerAddress;
use chumsky::prelude::*;
use crate::ast::tree::Slot;

pub fn literal<'a>() -> impl DatexParserTrait<'a> {
    // TODO: avoid repeating the map_with
    choice((
        select! { Token::True => DatexExpressionData::Boolean(true) }
            .map_with(|data, e| data.with_span(e.span())),
        select! { Token::False => DatexExpressionData::Boolean(false) }
            .map_with(|data, e| data.with_span(e.span())),
        select! { Token::Null => DatexExpressionData::Null }
            .map_with(|data, e| data.with_span(e.span())),
        // TODO #353: Remove clippy ignore
        select! { Token::NamedSlot(s) => DatexExpressionData::Slot(Slot::Named(s[1..].to_string())) }
            .map_with(|data, e| data.with_span(e.span())),
        select! { Token::PointerAddress(s) => DatexExpressionData::PointerAddress(PointerAddress::try_from(&s[1..]).unwrap()) }
            .map_with(|data, e| data.with_span(e.span())),
        select! { Token::Slot(s) => DatexExpressionData::Slot(Slot::Addressed(s[1..].parse::<u32>().unwrap())) }
            .map_with(|data, e| data.with_span(e.span())),
        select! { Token::Placeholder => DatexExpressionData::Placeholder }
            .map_with(|data, e| data.with_span(e.span())),
        select! { Token::Identifier(name) => DatexExpressionData::Identifier(name) }
            .map_with(|data, e| data.with_span(e.span())),
    ))
}
