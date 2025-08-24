use std::str::FromStr;

use crate::ast::DatexExpression;
use crate::ast::DatexParserTrait;
use crate::ast::ParserRecoverExt;
use crate::ast::error::error::ParseError;
use crate::compiler::lexer::Token;
use crate::values::core_values::endpoint::Endpoint;
use chumsky::prelude::*;

pub fn endpoint<'a>() -> impl DatexParserTrait<'a> {
    select! {
        Token::Endpoint(s) =>
            match Endpoint::from_str(s.as_str()) {
                Err(e) => Err(ParseError::from(e).with_note(
                    "Make sure the endpoint only contains valid characters."
                )),
                Ok(endpoint) => Ok(DatexExpression::Endpoint(endpoint))
        }
    }
    .recover_invalid()
}
