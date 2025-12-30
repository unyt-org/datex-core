use crate::ast::lexer::Token;
use crate::ast::spanned::Spanned;
use crate::ast::structs::expression::{DatexExpression, DatexExpressionData, Statements};
use crate::parser::errors::SpannedParserError;
use crate::parser::Parser;

impl Parser {
    pub(crate) fn parse_statements(&mut self) -> Result<DatexExpression, SpannedParserError> {
        let start = self.expect(Token::LeftParen)?.span.start;
        let mut statements = Vec::new();
        let mut is_terminated = false;

        while self.peek()?.token != Token::RightParen {

            // semicolons before statement
            while self.peek()?.token == Token::Semicolon {
                self.advance()?;
            }

            // if already at right paren without any statement, break
            if self.peek()?.token == Token::RightParen {
                break;
            }

            // parse statement
            statements.push(self.parse_expression(0)?);

            // semicolons after statement
            is_terminated = false;
            while self.peek()?.token == Token::Semicolon {
                self.advance()?;
                is_terminated = true;
            }
        }

        let end = self.expect(Token::RightParen)?.span.end;
        Ok(
            DatexExpressionData::Statements(Statements {
                statements,
                is_terminated,
                unbounded: None,
            }).with_span(start..end)
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::ast::spanned::Spanned;
    use crate::ast::structs::expression::{DatexExpressionData, Statements};
    use crate::parser::tests::{parse, try_parse_and_return_on_first_error};

    #[test]
    fn parse_empty_statements() {
        let expr = parse("()");
        assert_eq!(expr.data, DatexExpressionData::Statements(Statements { statements: vec![], is_terminated: false, unbounded: None }));
    }

    #[test]
    fn parse_simple_statements() {
        let expr = parse("(true; false; null;)");
        assert_eq!(expr.data, DatexExpressionData::Statements(Statements {
            statements: vec![
                DatexExpressionData::Boolean(true).with_default_span(),
                DatexExpressionData::Boolean(false).with_default_span(),
                DatexExpressionData::Null.with_default_span(),
            ],
            is_terminated: true,
            unbounded: None,
        }));
    }

    #[test]
    fn parse_simple_unterminated_statements() {
        let expr = parse("(true; false; null)");
        assert_eq!(expr.data, DatexExpressionData::Statements(Statements {
            statements: vec![
                DatexExpressionData::Boolean(true).with_default_span(),
                DatexExpressionData::Boolean(false).with_default_span(),
                DatexExpressionData::Null.with_default_span(),
            ],
            is_terminated: false,
            unbounded: None,
        }));
    }

    #[test]
    fn parse_statements_with_no_statements_but_terminated() {
        let expr = parse("(;)");
        assert_eq!(expr.data, DatexExpressionData::Statements(Statements {
            statements: vec![],
            is_terminated: false,
            unbounded: None,
        }));
    }

    fn parse_statements_with_multiple_semicolons() {
        let expr = parse("(;;true;;; false;; ; null;)");
        assert_eq!(expr.data, DatexExpressionData::Statements(Statements {
            statements: vec![
                DatexExpressionData::Boolean(true).with_default_span(),
                DatexExpressionData::Boolean(false).with_default_span(),
                DatexExpressionData::Null.with_default_span(),
            ],
            is_terminated: true,
            unbounded: None,
        }));
    }

    #[test]
    fn parse_statements_with_span() {
        let expr = parse("()");
        assert_eq!(expr.span, 0..2);

        let expr = parse("(true; false)");
        assert_eq!(expr.span, 0..13);
    }
}