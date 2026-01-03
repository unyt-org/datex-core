use crate::ast::spanned::Spanned;
use crate::ast::expressions::{
    DatexExpression, DatexExpressionData, List,
};
use crate::parser::lexer::Token;
use crate::parser::{Parser, SpannedParserError};

impl Parser {
    pub fn parse_list(
        &mut self,
    ) -> Result<DatexExpression, SpannedParserError> {
        let start = self.expect(Token::LeftBracket)?.span.start;
        let mut items = Vec::new();

        while self.peek()?.token != Token::RightBracket {
            let maybe_expression = self.parse_expression(0);
            let expression = self.recover_on_error(
                maybe_expression,
                &[Token::Comma, Token::RightBracket],
            )?;
            items.push(expression);

            if self.peek()?.token == Token::Comma {
                self.advance()?;
            }
        }

        let end = self.expect(Token::RightBracket)?.span.end;
        Ok(DatexExpressionData::List(List { items }).with_span(start..end))
    }
}

#[cfg(test)]
mod tests {
    use crate::ast::spanned::Spanned;
    use crate::ast::expressions::{DatexExpressionData, List};
    use crate::parser::errors::ParserError;
    use crate::parser::lexer::Token;
    use crate::parser::parser_result::ParserResult;
    use crate::parser::tests::{
        parse, try_parse_and_collect_errors,
        try_parse_and_return_on_first_error,
    };
    use core::assert_matches::assert_matches;

    #[test]
    fn parse_empty_list() {
        let expr = parse("[]");
        assert_eq!(
            expr.data,
            DatexExpressionData::List(List { items: vec![] })
        );
    }

    #[test]
    fn parse_simple_list() {
        let expr = parse("[true, false, null]");
        assert_eq!(
            expr.data,
            DatexExpressionData::List(List {
                items: vec![
                    DatexExpressionData::Boolean(true).with_default_span(),
                    DatexExpressionData::Boolean(false).with_default_span(),
                    DatexExpressionData::Null.with_default_span(),
                ]
            })
        );
    }

    #[test]
    fn parse_list_with_trailing_comma() {
        let expr = parse("[true, false, null,]");
        assert_eq!(
            expr.data,
            DatexExpressionData::List(List {
                items: vec![
                    DatexExpressionData::Boolean(true).with_default_span(),
                    DatexExpressionData::Boolean(false).with_default_span(),
                    DatexExpressionData::Null.with_default_span(),
                ]
            })
        );
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

    #[test]
    fn parse_recover_from_error() {
        let res = try_parse_and_collect_errors("[true, x + , false]");
        if let ParserResult::Invalid(res) = res {
            assert_matches!(
                res.errors[0].error,
                ParserError::UnexpectedToken {
                    found: Token::Comma,
                    ..
                }
            );
            assert_eq!(
                res.ast.data,
                DatexExpressionData::List(List {
                    items: vec![
                        DatexExpressionData::Boolean(true).with_default_span(),
                        DatexExpressionData::Recover.with_default_span(),
                        DatexExpressionData::Boolean(false).with_default_span(),
                    ]
                })
            );
        } else {
            panic!("Expected invalid parser result");
        }
    }

    #[test]
    fn parse_recover_from_error_end() {
        let res = try_parse_and_collect_errors("[true, x +]");
        if let ParserResult::Invalid(res) = res {
            assert_matches!(
                res.errors[0].error,
                ParserError::UnexpectedToken {
                    found: Token::RightBracket,
                    ..
                }
            );
            assert_eq!(
                res.ast.data,
                DatexExpressionData::List(List {
                    items: vec![
                        DatexExpressionData::Boolean(true).with_default_span(),
                        DatexExpressionData::Recover.with_default_span(),
                    ]
                })
            );
        } else {
            panic!("Expected invalid parser result");
        }
    }
}
