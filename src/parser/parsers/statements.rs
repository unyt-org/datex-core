use crate::parser::lexer::Token;
use crate::ast::spanned::Spanned;
use crate::ast::structs::expression::{DatexExpression, DatexExpressionData, Statements};
use crate::parser::errors::SpannedParserError;
use crate::parser::Parser;

impl Parser {
    pub(crate) fn parse_parenthesized_statements(&mut self) -> Result<DatexExpression, SpannedParserError> {
        let start = self.expect(Token::LeftParen)?.span.start;
        let mut statements_data = self.parse_statements()?;

        let end = self.expect(Token::RightParen)?.span.end;
        Ok(statements_data.data.with_span(start..end))
    }

    pub(crate) fn parse_statements(&mut self) -> Result<DatexExpression, SpannedParserError> {
        let mut statements = Vec::new();
        let mut is_terminated = false;

        loop {
            if !self.has_more_tokens() {
                break;
            }
            match self.peek()?.token {
                Token::Semicolon => {
                    self.advance()?;
                    is_terminated = true;
                },
                Token::RightParen => break,
                _ => {
                    is_terminated = false;
                    statements.push(self.parse_statement()?);
                }
            }
        }
        
        // if single statement and not terminated, return that statement directly
        if statements.len() == 1 && !is_terminated {
            Ok(statements.remove(0))
        }
        // otherwise, return as statements 
        else {
            Ok(DatexExpressionData::Statements(Statements {
                statements,
                is_terminated,
                unbounded: None,
            }).with_default_span())
        }
    }

    pub(crate) fn parse_top_level_statements(&mut self) -> Result<DatexExpression, SpannedParserError> {
        let statements_data = self.parse_statements()?;

        Ok(match statements_data.data {
            // if statements expression, set span correctly
            DatexExpressionData::Statements(_) => {
                let full_token_span = 0..self.tokens.last().map(|i| i.span.end).unwrap_or(0);
                statements_data.data.with_span(full_token_span)
            },
            // otherwise, just return as is
            _ => {
                statements_data
            }
        })
    }

    fn parse_statement(&mut self) -> Result<DatexExpression, SpannedParserError> {
        match self.peek()?.token {
            Token::Variable | Token::Const => self.parse_variable_declaration(),
            Token::Type | Token::TypeAlias => self.parse_type_declaration(),
            _ => self.parse_expression(0)
        }
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
            is_terminated: true,
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

    #[test]
    fn top_level_statements() {
        let expr = parse("true; false; null;");
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
    fn top_level_single_statement_unterminated() {
        let expr = parse("true");
        assert_eq!(expr.data, DatexExpressionData::Boolean(true));
    }
    
    #[test]
    fn top_level_single_statement_terminated() {
        let expr = parse("true;");
        assert_eq!(expr.data, DatexExpressionData::Statements(Statements {
            statements: vec![
                DatexExpressionData::Boolean(true).with_default_span(),
            ],
            is_terminated: true,
            unbounded: None,
        }));
    }
}