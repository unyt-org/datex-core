use datex_core::ast::structs::r#type::TypeExpression;
use crate::ast::spanned::Spanned;
use crate::ast::lexer::Token;
use crate::ast::structs::r#type::{StructuralList, TypeExpressionData};
use crate::parser::{SpannedParserError, Parser};

impl Parser {
    pub fn parse_type_list(&mut self) -> Result<TypeExpression, SpannedParserError> {
        let start = self.expect(Token::LeftBracket)?.span.start;
        let mut items = Vec::new();

        while self.peek()?.token != Token::RightBracket {
            items.push(self.parse_type_expression(0)?);

            if self.peek()?.token == Token::Comma {
                self.advance()?;
            }
        }

        let end = self.expect(Token::RightBracket)?.span.end;
        Ok(
            TypeExpressionData::StructuralList(StructuralList(items))
                .with_span(start..end)
        )
    }
}

#[cfg(test)]
mod tests {
    use datex_core::ast::structs::r#type::TypeExpressionData;
    use crate::ast::spanned::Spanned;
    use crate::ast::structs::r#type::StructuralList;
    use crate::parser::parsers::type_expressions::tests::parse_type_expression;

    #[test]
    fn parse_empty_list() {
        let expr = parse_type_expression("[]");
        assert_eq!(expr.data, TypeExpressionData::StructuralList(StructuralList(vec![])));
    }

    #[test]
    fn parse_simple_list() {
        let expr = parse_type_expression("[true, false, null]");
        assert_eq!(expr.data, TypeExpressionData::StructuralList(StructuralList(vec![
            TypeExpressionData::Boolean(true).with_default_span(),
            TypeExpressionData::Boolean(false).with_default_span(),
            TypeExpressionData::Null.with_default_span(),
        ])));
    }

    #[test]
    fn parse_list_with_trailing_comma() {
        let expr = parse_type_expression("[true, false, null,]");
        assert_eq!(expr.data, TypeExpressionData::StructuralList(StructuralList(vec![
            TypeExpressionData::Boolean(true).with_default_span(),
            TypeExpressionData::Boolean(false).with_default_span(),
            TypeExpressionData::Null.with_default_span(),
        ])));
    }
}