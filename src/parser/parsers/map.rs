use crate::ast::spanned::Spanned;
use crate::parser::lexer::Token;
use crate::ast::structs::expression::{DatexExpression, DatexExpressionData, Map};
use crate::parser::{SpannedParserError, Parser};

impl Parser {
    pub fn parse_map(&mut self) -> Result<DatexExpression, SpannedParserError> {
        let start = self.expect(Token::LeftCurly)?.span.start;
        let mut entries = Vec::new();

        while self.peek()?.token != Token::RightCurly {
            let key = self.parse_key()?;
            self.expect(Token::Colon)?;
            let value = self.parse_expression(0)?;
            entries.push((key, value));

            if self.peek()?.token == Token::Comma {
                self.advance()?;
            }
        }
        let end = self.expect(Token::RightCurly)?.span.end;
        Ok(
            DatexExpressionData::Map(Map {
                entries
            }).with_span(start..end)
        )
    }
}

#[cfg(test)]
mod tests {
    use datex_core::ast::structs::expression::BinaryOperation;
    use crate::ast::spanned::Spanned;
    use crate::ast::structs::expression::{DatexExpressionData, Map};
    use crate::global::operators::binary::ArithmeticOperator;
    use crate::global::operators::BinaryOperator;
    use crate::parser::tests::{parse, try_parse_and_return_on_first_error};

    #[test]
    fn parse_empty_map() {
        let expr = parse("{}");
        assert_eq!(expr.data, DatexExpressionData::Map(Map { entries: vec![] }));
    }

    #[test]
    fn parse_simple_map() {
        let expr = parse("{'key1': true, 'key2': false}");
        assert_eq!(expr.data, DatexExpressionData::Map(Map {
            entries: vec![
                (DatexExpressionData::Text("key1".to_string()).with_default_span(), DatexExpressionData::Boolean(true).with_default_span()),
                (DatexExpressionData::Text("key2".to_string()).with_default_span(), DatexExpressionData::Boolean(false).with_default_span()),
            ]
        }));
    }
    
    #[test]
    fn parse_map_with_plain_identifier_keys() {
        let expr = parse("{key1: true, key2: false}");
        assert_eq!(expr.data, DatexExpressionData::Map(Map {
            entries: vec![
                (DatexExpressionData::Text("key1".to_string()).with_default_span(), DatexExpressionData::Boolean(true).with_default_span()),
                (DatexExpressionData::Text("key2".to_string()).with_default_span(), DatexExpressionData::Boolean(false).with_default_span()),
            ]
        }));
    }
    
    #[test]
    fn parse_map_with_dynamic_expression_keys() {
        let expr = parse("{(x): true, (y + true): false}");
        assert_eq!(expr.data, DatexExpressionData::Map(Map {
            entries: vec![
                (
                    DatexExpressionData::Identifier("x".to_string()).with_default_span(),
                    DatexExpressionData::Boolean(true).with_default_span()
                ),
                (
                    DatexExpressionData::BinaryOperation(BinaryOperation {
                        left: Box::new(DatexExpressionData::Identifier("y".to_string()).with_default_span()),
                        operator: BinaryOperator::Arithmetic(ArithmeticOperator::Add),
                        right: Box::new(DatexExpressionData::Boolean(true).with_default_span()),
                        ty: None,
                    }).with_default_span(),
                    DatexExpressionData::Boolean(false).with_default_span()
                ),
            ]
        }));
    }
}