use crate::ast::spanned::Spanned;
use crate::parser::lexer::Token;
use crate::ast::structs::expression::{DatexExpression, DatexExpressionData, Map};
use crate::ast::structs::r#type::{TypeExpression, TypeExpressionData};
use crate::parser::{SpannedParserError, Parser};

impl Parser {
    pub(crate) fn parse_type_grouped(&mut self) -> Result<TypeExpression, SpannedParserError> {
        let start = self.expect(Token::LeftParen)?.span.start;
        let mut inner_expression = self.parse_type_expression(0)?;

        let end = self.expect(Token::RightParen)?.span.end;
        Ok(inner_expression.data.with_span(start..end))
    }
}