use crate::ast::spanned::Spanned;
use crate::ast::lexer::Token;
use crate::ast::structs::expression::{DatexExpression, DatexExpressionData, Map};
use crate::parser::{SpannedParserError, Parser};

impl Parser {
    pub(crate) fn parse_key(&mut self) -> Result<DatexExpression, SpannedParserError> {
        Ok(match self.peek()?.token.clone() {

            // treat plain identifiers as text keys
            Token::Identifier(name) => {
                DatexExpressionData::Text(name)
                    .with_span(self.advance()?.span)
            }

            // treat everything else as normal atom
            _ => self.parse_atom()?,
        })
    }
}