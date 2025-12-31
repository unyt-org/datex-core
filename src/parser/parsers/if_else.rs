use crate::ast::DatexExpressionData;
use crate::ast::spanned::Spanned;
use crate::ast::structs::expression::{Conditional, DatexExpression};
use crate::parser::lexer::Token;
use crate::parser::{Parser, SpannedParserError};

impl Parser {
    pub(crate) fn parse_if_else(
        &mut self,
    ) -> Result<DatexExpression, SpannedParserError> {
        let start = self.expect(Token::If)?.span.start;

        let condition = self.parse_parenthesized_statements()?;
        let then_branch = self.parse_parenthesized_statements()?;

        let else_branch = if let Ok(token) = self.peek()
            && token.token == Token::Else
        {
            self.advance()?;
            Some(Box::new(self.parse_parenthesized_statements_or_if_else()?))
        } else {
            None
        };

        Ok(DatexExpressionData::Conditional(Conditional {
            condition: Box::new(condition),
            then_branch: Box::new(then_branch),
            else_branch,
        })
        .with_span(start..self.get_current_source_position()))
    }

    fn parse_parenthesized_statements_or_if_else(
        &mut self,
    ) -> Result<DatexExpression, SpannedParserError> {
        if let Ok(token) = self.peek()
            && token.token == Token::If
        {
            self.parse_if_else()
        } else {
            self.parse_parenthesized_statements()
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::ast::spanned::Spanned;
    use crate::ast::structs::expression::{Conditional, DatexExpressionData};
    use crate::parser::tests::parse;

    #[test]
    fn parse_single_if() {
        let expr = parse("if (true) (42)");
        assert_eq!(
            expr.data,
            DatexExpressionData::Conditional(Conditional {
                condition: Box::new(
                    DatexExpressionData::Boolean(true).with_default_span()
                ),
                then_branch: Box::new(
                    DatexExpressionData::Integer(42.into()).with_default_span()
                ),
                else_branch: None,
            })
        )
    }

    #[test]
    fn parse_if_else() {
        let expr = parse("if (false) (0) else (1)");
        assert_eq!(
            expr.data,
            DatexExpressionData::Conditional(Conditional {
                condition: Box::new(
                    DatexExpressionData::Boolean(false).with_default_span()
                ),
                then_branch: Box::new(
                    DatexExpressionData::Integer(0.into()).with_default_span()
                ),
                else_branch: Some(Box::new(
                    DatexExpressionData::Integer(1.into()).with_default_span()
                )),
            })
        )
    }

    #[test]
    fn parse_nested_if_else() {
        let expr = parse("if (true) (1) else if (false) (2) else (3)");
        assert_eq!(
            expr.data,
            DatexExpressionData::Conditional(Conditional {
                condition: Box::new(
                    DatexExpressionData::Boolean(true).with_default_span()
                ),
                then_branch: Box::new(
                    DatexExpressionData::Integer(1.into()).with_default_span()
                ),
                else_branch: Some(Box::new(
                    DatexExpressionData::Conditional(Conditional {
                        condition: Box::new(
                            DatexExpressionData::Boolean(false)
                                .with_default_span()
                        ),
                        then_branch: Box::new(
                            DatexExpressionData::Integer(2.into())
                                .with_default_span()
                        ),
                        else_branch: Some(Box::new(
                            DatexExpressionData::Integer(3.into())
                                .with_default_span()
                        )),
                    })
                    .with_default_span()
                )),
            })
        )
    }
}
