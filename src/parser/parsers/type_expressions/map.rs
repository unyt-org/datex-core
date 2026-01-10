use crate::ast::spanned::Spanned;
use crate::ast::type_expressions::TypeExpression;
use crate::ast::type_expressions::{StructuralMap, TypeExpressionData};
use crate::parser::lexer::Token;
use crate::parser::{Parser, SpannedParserError};

impl Parser {
    pub fn parse_type_map(
        &mut self,
    ) -> Result<TypeExpression, SpannedParserError> {
        let start = self.expect(Token::LeftCurly)?.span.start;
        let mut entries = Vec::new();

        while self.peek()?.token != Token::RightCurly {
            let key = self.parse_type_key()?;
            self.expect(Token::Colon)?;
            let value = self.parse_type_expression(0)?;
            entries.push((key, value));

            if self.peek()?.token == Token::Comma {
                self.advance()?;
            }
        }
        let end = self.expect(Token::RightCurly)?.span.end;
        Ok(TypeExpressionData::StructuralMap(StructuralMap(entries))
            .with_span(start..end))
    }
}

#[cfg(test)]
mod tests {
    use crate::ast::spanned::Spanned;
    use crate::ast::type_expressions::StructuralMap;
    use crate::ast::type_expressions::{Intersection, TypeExpressionData};
    use crate::parser::parsers::type_expressions::tests::parse_type_expression;

    #[test]
    fn parse_empty_map() {
        let expr = parse_type_expression("{}");
        assert_eq!(
            expr.data,
            TypeExpressionData::StructuralMap(StructuralMap(vec![]))
        );
    }

    #[test]
    fn parse_simple_map() {
        let expr = parse_type_expression("{'key1': true, 'key2': false}");
        assert_eq!(
            expr.data,
            TypeExpressionData::StructuralMap(StructuralMap(vec![
                (
                    TypeExpressionData::Text("key1".to_string())
                        .with_default_span(),
                    TypeExpressionData::Boolean(true).with_default_span()
                ),
                (
                    TypeExpressionData::Text("key2".to_string())
                        .with_default_span(),
                    TypeExpressionData::Boolean(false).with_default_span()
                ),
            ]))
        );
    }

    #[test]
    fn parse_map_with_plain_identifier_keys() {
        let expr = parse_type_expression("{key1: true, key2: false}");
        assert_eq!(
            expr.data,
            TypeExpressionData::StructuralMap(StructuralMap(vec![
                (
                    TypeExpressionData::Text("key1".to_string())
                        .with_default_span(),
                    TypeExpressionData::Boolean(true).with_default_span()
                ),
                (
                    TypeExpressionData::Text("key2".to_string())
                        .with_default_span(),
                    TypeExpressionData::Boolean(false).with_default_span()
                ),
            ]))
        );
    }

    #[test]
    fn parse_map_with_reserved_keyword_keys() {
        let expr = parse_type_expression("{if: true, type: false}");
        assert_eq!(
            expr.data,
            TypeExpressionData::StructuralMap(StructuralMap(vec![
                (
                    TypeExpressionData::Text("if".to_string())
                        .with_default_span(),
                    TypeExpressionData::Boolean(true).with_default_span()
                ),
                (
                    TypeExpressionData::Text("type".to_string())
                        .with_default_span(),
                    TypeExpressionData::Boolean(false).with_default_span()
                ),
            ]))
        );
    }

    #[test]
    fn parse_map_with_dynamic_expression_keys() {
        let expr = parse_type_expression("{(x): true, (y & true): false}");
        assert_eq!(
            expr.data,
            TypeExpressionData::StructuralMap(StructuralMap(vec![
                (
                    TypeExpressionData::Identifier("x".to_string())
                        .with_default_span(),
                    TypeExpressionData::Boolean(true).with_default_span()
                ),
                (
                    TypeExpressionData::Intersection(Intersection(vec![
                        TypeExpressionData::Identifier("y".to_string())
                            .with_default_span(),
                        TypeExpressionData::Boolean(true).with_default_span(),
                    ]))
                    .with_default_span(),
                    TypeExpressionData::Boolean(false).with_default_span(),
                )
            ]))
        );
    }
}
