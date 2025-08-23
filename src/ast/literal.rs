use crate::compiler::ast_parser::{DatexExpression, Slot};
use crate::compiler::lexer::Token;
use chumsky::extra::Err;
use chumsky::prelude::*;

pub fn literal<'a>()
-> impl Parser<'a, &'a [Token], DatexExpression, Err<Cheap>> + Clone + 'a {
    choice((
        select! { Token::True => DatexExpression::Boolean(true) },
        select! { Token::False => DatexExpression::Boolean(false) },
        select! { Token::Null => DatexExpression::Null },
        select! { Token::NamedSlot(s) => DatexExpression::Slot(Slot::Named(s[1..].to_string())) },
        select! { Token::Slot(s) => DatexExpression::Slot(Slot::Addressed(s[1..].parse::<u32>().unwrap())) },
        select! { Token::Placeholder => DatexExpression::Placeholder },
        select! { Token::Identifier(name) => name }
            .then(
                just(Token::Slash)
                    .ignore_then(select! { Token::Identifier(sub) => sub })
                    .or_not(),
            )
            .map(|(name, variant)| DatexExpression::Literal { name, variant }),
    ))
}
