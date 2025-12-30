use crate::ast::spanned::Spanned;
use crate::parser::lexer::Token;
use crate::ast::structs::expression::{DatexExpression, DatexExpressionData, List};
use crate::parser::{SpannedParserError, Parser};

impl Parser {
    pub fn parse_list(&mut self) -> Result<DatexExpression, SpannedParserError> {
        let start = self.expect(Token::LeftBracket)?.span.start;
        let mut items = Vec::new();

        while self.peek()?.token != Token::RightBracket {
            items.push(self.parse_expression(0)?);

            if self.peek()?.token == Token::Comma {
                self.advance()?;
            }
        }

        let end = self.expect(Token::RightBracket)?.span.end;
        Ok(
            DatexExpressionData::List(List {
                items
            }).with_span(start..end)
        )
    }
}

#[cfg(test)]
mod tests {
    use std::assert_matches::assert_matches;
    use crate::parser::lexer::Token;
    use crate::ast::spanned::Spanned;
    use crate::ast::structs::expression::{DatexExpressionData, List};
    use crate::parser::errors::ParserError;
    use crate::parser::tests::{parse, try_parse_and_return_on_first_error};

    #[test]
    fn parse_empty_list() {
        let expr = parse("[]");
        assert_eq!(expr.data, DatexExpressionData::List(List { items: vec![] }));
    }

    #[test]
    fn parse_simple_list() {
        let expr = parse("[true, false, null]");
        assert_eq!(expr.data, DatexExpressionData::List(List {
            items: vec![
                DatexExpressionData::Boolean(true).with_default_span(),
                DatexExpressionData::Boolean(false).with_default_span(),
                DatexExpressionData::Null.with_default_span(),
            ]
        }));
    }

    #[test]
    fn parse_list_with_trailing_comma() {
        let expr = parse("[true, false, null,]");
        assert_eq!(expr.data, DatexExpressionData::List(List {
            items: vec![
                DatexExpressionData::Boolean(true).with_default_span(),
                DatexExpressionData::Boolean(false).with_default_span(),
                DatexExpressionData::Null.with_default_span(),
            ]
        }));
    }

    #[test]
    fn parse_list_with_wrong_close_paren() {
        let result = try_parse_and_return_on_first_error("[true}");
        assert!(result.is_err());
        assert_matches!(
            result.err().unwrap().error,
            ParserError::UnexpectedToken {
                found: Token::RightCurly,
                ..
            }
        );
    }

    #[test]
    fn parse_with_span() {
        let expr = parse("[]");
        assert_eq!(expr.span, 0..2);

        let expr = parse("[true, false]");
        assert_eq!(expr.span, 0..13);

        let expr = parse(" [  true , false ] ");
        assert_eq!(expr.span, 1..18);
    }
}