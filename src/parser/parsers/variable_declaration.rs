use crate::ast::spanned::Spanned;
use crate::parser::lexer::{SpannedToken, Token};
use crate::ast::structs::expression::{DatexExpression, DatexExpressionData, VariableDeclaration, VariableKind};
use crate::parser::{SpannedParserError, Parser};
use crate::parser::errors::ParserError;

impl Parser {
    pub(crate) fn parse_variable_declaration(&mut self) -> Result<DatexExpression, SpannedParserError> {
        Ok(match self.peek()?.token.clone() {

            // handle var and const declarations
            Token::Variable | Token::Const => {
                let kind = match self.advance()?.token {
                    Token::Variable => VariableKind::Var,
                    Token::Const => VariableKind::Const,
                    _ => unreachable!()
                };

                let (name, _) = self.expect_identifier()?;

                // optional type annotation if followed by colon
                let type_annotation = match self.peek()?.token {
                    Token::Colon => {
                        self.advance()?; // consume colon
                        Some(self.parse_type_expression(0)?)
                    }
                    _ => None
                };

                // expect equals sign
                self.expect(Token::Assign)?;

                // initializer expression
                let init_expression = self.parse_expression(0)?;

                DatexExpressionData::VariableDeclaration(VariableDeclaration {
                    id: None,
                    kind,
                    name,
                    type_annotation,
                    init_expression: Box::new(init_expression),
                })
                .with_default_span()
            }

            _ => return Err(SpannedParserError {
                error: ParserError::UnexpectedToken {
                    expected: vec![
                        Token::Variable,
                        Token::Const,
                    ],
                    found: self.peek()?.token.clone(),
                },
                span: self.peek()?.span.clone()
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::ast::spanned::Spanned;
    use crate::ast::structs::expression::{DatexExpressionData, VariableDeclaration, VariableKind};
    use crate::ast::structs::r#type::{TypeExpressionData, Union};
    use crate::parser::tests::{parse, try_parse_and_return_on_first_error};

    #[test]
    fn parse_variable_declaration_var() {
        let expr = parse("var myVar = true");
        assert_eq!(expr.data, DatexExpressionData::VariableDeclaration(VariableDeclaration {
            id: None,
            kind: VariableKind::Var,
            name: "myVar".to_string(),
            type_annotation: None,
            init_expression: Box::new(DatexExpressionData::Boolean(true).with_default_span()),
        }));
    }

    #[test]
    fn parse_variable_declaration_const() {
        let expr = parse("const myConst = false");
        assert_eq!(expr.data, DatexExpressionData::VariableDeclaration(VariableDeclaration {
            id: None,
            kind: VariableKind::Const,
            name: "myConst".to_string(),
            type_annotation: None,
            init_expression: Box::new(DatexExpressionData::Boolean(false).with_default_span()),
        }));
    }

    #[test]
    fn parse_variable_declaration_with_type_annotation() {
        let expr = parse("var myVar: boolean = true");
        assert_eq!(expr.data, DatexExpressionData::VariableDeclaration(VariableDeclaration {
            id: None,
            kind: VariableKind::Var,
            name: "myVar".to_string(),
            type_annotation: Some(
                TypeExpressionData::Identifier("boolean".to_string()).with_default_span()
            ),
            init_expression: Box::new(DatexExpressionData::Boolean(true).with_default_span()),
        }));
    }

    #[test]
    fn parse_variable_declaration_with_complex_type_annotation() {
        let expr = parse("const myConst: integer|text = true");
        assert_eq!(expr.data, DatexExpressionData::VariableDeclaration(VariableDeclaration {
            id: None,
            kind: VariableKind::Const,
            name: "myConst".to_string(),
            type_annotation: Some(
                TypeExpressionData::Union(Union(
                    vec![
                        TypeExpressionData::Identifier("integer".to_string()).with_default_span(),
                        TypeExpressionData::Identifier("text".to_string()).with_default_span(),
                    ]
                )).with_default_span()),
            init_expression: Box::new(DatexExpressionData::Boolean(true).with_default_span()),
        }));
    }
}