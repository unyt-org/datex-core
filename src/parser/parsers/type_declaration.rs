use crate::ast::spanned::Spanned;
use crate::ast::lexer::{SpannedToken, Token};
use crate::ast::structs::expression::{DatexExpression, DatexExpressionData, TypeDeclaration, TypeDeclarationKind, VariableDeclaration, VariableKind};
use crate::parser::{SpannedParserError, Parser};
use crate::parser::errors::ParserError;

impl Parser {
    pub(crate) fn parse_type_declaration(&mut self) -> Result<DatexExpression, SpannedParserError> {
        Ok(match self.peek()?.token.clone() {

            // handle var and const declarations
            Token::Type | Token::TypeAlias => {
                let kind = match self.advance()?.token {
                    Token::Type => TypeDeclarationKind::Nominal,
                    Token::TypeAlias => TypeDeclarationKind::Structural,
                    _ => unreachable!()
                };
                
                let name = self.expect_identifier()?;

                // expect equals sign
                self.expect(Token::Assign)?;

                // initializer expression
                let definition = self.parse_type_expression(0)?;

                DatexExpressionData::TypeDeclaration(TypeDeclaration {
                    id: None,
                    kind,
                    name,
                    definition,
                    hoisted: false,
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
    use crate::ast::structs::expression::{DatexExpressionData, TypeDeclaration, TypeDeclarationKind, VariableDeclaration, VariableKind};
    use crate::ast::structs::r#type::TypeExpressionData;
    use crate::parser::tests::{parse, try_parse_and_return_on_first_error};

    #[test]
    fn parse_type_declaration() {
        let expr = parse("type myType = true");
        assert_eq!(expr.data, DatexExpressionData::TypeDeclaration(TypeDeclaration {
            id: None,
            kind: TypeDeclarationKind::Nominal,
            name: "myType".to_string(),
            definition: TypeExpressionData::Boolean(true).with_default_span(),
            hoisted: false,
        }));
    }

    #[test]
    fn parse_type_alias_declaration() {
        let expr = parse("typealias myAlias = false");
        assert_eq!(expr.data, DatexExpressionData::TypeDeclaration(TypeDeclaration {
            id: None,
            kind: TypeDeclarationKind::Structural,
            name: "myAlias".to_string(),
            definition: TypeExpressionData::Boolean(false).with_default_span(),
            hoisted: false,
        }));
    }
}