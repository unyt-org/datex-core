use crate::ast::DatexExpressionData;
use crate::ast::DatexParserTrait;
use crate::ast::ParserRecoverExt;
use crate::ast::error::error::ParseError;
use crate::ast::lexer::Token;
use crate::ast::spanned::Spanned;
use crate::values::core_values::endpoint::Endpoint;
use chumsky::prelude::*;
use core::str::FromStr;

pub fn endpoint<'a>() -> impl DatexParserTrait<'a> {
    select! {
        Token::Endpoint(s) =>
            match Endpoint::from_str(s.as_str()) {
                Err(e) => Err(ParseError::from(e).with_note(
                    "Make sure the endpoint only contains valid characters."
                )),
                Ok(endpoint) => Ok(DatexExpressionData::Endpoint(endpoint))
        }
    }
    .map_with(|data, e| data.map(|data| data.with_span(e.span())))
    .recover_invalid()
}
