use crate::ast::error::pattern::Pattern;
use crate::ast::lexer::Token;
use crate::ast::utils::whitespace;
use crate::ast::{DatexExpression, DatexParserTrait};

use crate::ast::text::unescape_text;
use chumsky::prelude::*;

pub fn structure<'a>(
    expression_without_list: impl DatexParserTrait<'a>,
) -> impl DatexParserTrait<'a> {
    let key = choice((
        select! {
            Token::StringLiteral(s) => unescape_text(&s)
        },
        // any valid identifiers (equivalent to variable names), mapped to a text
        select! {
            Token::Identifier(s) => s
        },
    ));
    key.then_ignore(just(Token::Colon).padded_by(whitespace()))
        .then(expression_without_list.clone())
        .separated_by(just(Token::Comma).padded_by(whitespace()))
        .at_least(0)
        .allow_trailing()
        .collect()
        .padded_by(whitespace())
        .delimited_by(just(Token::LeftCurly), just(Token::RightCurly))
        .map(DatexExpression::Struct)
        .labelled(Pattern::Custom("struct"))
        .as_context()
}
