use crate::ast::spanned::Spanned;
use crate::ast::expressions::{
    DatexExpression, DatexExpressionData, TypeDeclaration, TypeDeclarationKind,
    VariableDeclaration, VariableKind,
};
use crate::parser::errors::ParserError;
use crate::parser::lexer::{SpannedToken, Token};
use crate::parser::{Parser, SpannedParserError};

impl Parser {
    pub(crate) fn parse_type_declaration(
        &mut self,
    ) -> Result<DatexExpression, SpannedParserError> {
        Ok(match self.peek()?.token.clone() {
            // handle var and const declarations
            Token::TypeDeclaration | Token::TypeAlias => {
                let kind = match self.advance()?.token {
                    Token::TypeDeclaration => TypeDeclarationKind::Nominal,
                    Token::TypeAlias => TypeDeclarationKind::Structural,
                    _ => unreachable!(),
                };

                let (mut name, _) = self.expect_identifier()?;

                // optional /variant
                if self.peek()?.token == Token::Slash {
                    // consume slash
                    self.advance()?;
                    let (variant, _) = self.expect_identifier()?;
                    // append to name
                    let full_name = format!("{}/{}", name, variant);
                    name = full_name;
                }

                // optional generic parameters
                // TODO: use generic parameters
                let _generic_params = if self.peek()?.token == Token::LeftAngle
                {
                    Some(self.parse_generic_parameters()?)
                } else {
                    None
                };

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

            _ => {
                return Err(SpannedParserError {
                    error: ParserError::UnexpectedToken {
                        expected: vec![Token::Variable, Token::Const],
                        found: self.peek()?.token.clone(),
                    },
                    span: self.peek()?.span.clone(),
                });
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::ast::spanned::Spanned;
    use crate::ast::expressions::{
        DatexExpressionData, TypeDeclaration, TypeDeclarationKind,
        VariableDeclaration, VariableKind,
    };
    use crate::ast::type_expressions::TypeExpressionData;
    use crate::parser::tests::{parse, try_parse_and_return_on_first_error};

    #[test]
    fn parse_type_declaration() {
        let expr = parse("type myType = true");
        assert_eq!(
            expr.data,
            DatexExpressionData::TypeDeclaration(TypeDeclaration {
                id: None,
                kind: TypeDeclarationKind::Nominal,
                name: "myType".to_string(),
                definition: TypeExpressionData::Boolean(true)
                    .with_default_span(),
                hoisted: false,
            })
        );
    }

    #[test]
    fn parse_type_alias_declaration() {
        let expr = parse("typealias myAlias = false");
        assert_eq!(
            expr.data,
            DatexExpressionData::TypeDeclaration(TypeDeclaration {
                id: None,
                kind: TypeDeclarationKind::Structural,
                name: "myAlias".to_string(),
                definition: TypeExpressionData::Boolean(false)
                    .with_default_span(),
                hoisted: false,
            })
        );
    }

    #[test]
    fn parse_type_declaration_with_variant() {
        let expr = parse("type myType/variantA = null");
        assert_eq!(
            expr.data,
            DatexExpressionData::TypeDeclaration(TypeDeclaration {
                id: None,
                kind: TypeDeclarationKind::Nominal,
                name: "myType/variantA".to_string(),
                definition: TypeExpressionData::Null.with_default_span(),
                hoisted: false,
            })
        );
    }

    // TODO: generic parameters parsing
    #[test]
    fn parse_type_declaration_with_generic_parameters() {
        let expr = parse("type myType<T, U> = true");
        assert_eq!(
            expr.data,
            DatexExpressionData::TypeDeclaration(TypeDeclaration {
                id: None,
                kind: TypeDeclarationKind::Nominal,
                name: "myType".to_string(),
                definition: TypeExpressionData::Boolean(true)
                    .with_default_span(),
                hoisted: false,
            })
        );
    }

    #[test]
    fn parse_type_declaration_with_variant_and_generic_parameters() {
        let expr = parse("type myType/variantA<T> = false");
        assert_eq!(
            expr.data,
            DatexExpressionData::TypeDeclaration(TypeDeclaration {
                id: None,
                kind: TypeDeclarationKind::Nominal,
                name: "myType/variantA".to_string(),
                definition: TypeExpressionData::Boolean(false)
                    .with_default_span(),
                hoisted: false,
            })
        );
    }
}
