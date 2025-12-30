use crate::ast::spanned::Spanned;
use crate::parser::lexer::Token;
use crate::ast::structs::expression::{DatexExpression, DatexExpressionData, Map};
use crate::ast::structs::r#type::{TypeExpression, TypeExpressionData};
use crate::parser::{SpannedParserError, Parser};

impl Parser {
    pub(crate) fn parse_type_key(&mut self) -> Result<TypeExpression, SpannedParserError> {
        Ok(match self.peek()?.token.clone() {

            // treat plain identifiers as text keys
            Token::Identifier(name) => {
                TypeExpressionData::Text(name)
                    .with_span(self.advance()?.span)
            }

            // treat everything else as normal atom
            _ => self.parse_type_atom()?,
        })
    }
}