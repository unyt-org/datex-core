use std::str::FromStr;

use crate::ast::DatexExpression;
use crate::ast::DatexParserTrait;
use crate::ast::TokenInput;
use crate::compiler::lexer::Token;
use crate::values::core_values::endpoint::Endpoint;
use chumsky::extra::Err;
use chumsky::prelude::*;

pub fn endpoint<'a>() -> impl DatexParserTrait<'a> {
    select! {
        Token::Endpoint(s) =>
            match Endpoint::from_str(s.as_str()) {
                Err(_) => DatexExpression::Invalid,
                Ok(endpoint) => DatexExpression::Endpoint(endpoint)
        }
    }
}
