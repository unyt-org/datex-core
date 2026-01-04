mod atom;
mod grouped;
mod key;
mod list;
mod map;

use crate::ast::spanned::Spanned;
use crate::ast::type_expressions::{
    Intersection, TypeExpression, TypeExpressionData, TypeVariantAccess, Union,
};
use crate::global::operators::{ArithmeticUnaryOperator, UnaryOperator};
use crate::parser::Parser;
use crate::parser::errors::{ParserError, SpannedParserError};
use crate::parser::lexer::{SpannedToken, Token};
use crate::values::core_values::error::NumberParseError;

static UNARY_BP: u8 = 11; // less binding power than variant access and property access, but more than all other infix operators

impl Parser {
    pub(crate) fn parse_type_expression(
        &mut self,
        min_bp: u8,
    ) -> Result<TypeExpression, SpannedParserError> {
        let mut lhs = self.parse_type_prefix()?;

        while self.has_more_tokens() {
            let (_, r_bp) =
                match Parser::type_infix_binding_power(&self.peek()?.token) {
                    Some(bp) if bp.0 >= min_bp => bp,
                    _ => break,
                };

            let op = self.peek()?.clone();

            lhs = self.parse_type_binary_operation(lhs, op, r_bp)?;
        }

        Ok(lhs)
    }

    fn parse_type_binary_operation(
        &mut self,
        lhs: TypeExpression,
        op: SpannedToken,
        r_bp: u8,
    ) -> Result<TypeExpression, SpannedParserError> {
        Ok(match op.token {
            // union type operator
            Token::Pipe => {
                self.advance()?; // consume operator
                let rhs = self.parse_type_expression(r_bp)?;
                let span = lhs.span.start..rhs.span.end;
                TypeExpressionData::Union(Union(vec![lhs, rhs])).with_span(span)
            }
            // intersection type operator
            Token::Ampersand => {
                self.advance()?; // consume operator
                let rhs = self.parse_type_expression(r_bp)?;
                let span = lhs.span.start..rhs.span.end;
                TypeExpressionData::Intersection(Intersection(vec![lhs, rhs]))
                    .with_span(span)
            }

            // variant access operator (/)
            Token::Slash => {
                self.advance()?; // consume operator
                let (rhs, rhs_span) = self.expect_identifier()?;
                let span = lhs.span.start..rhs_span.end;

                // lhs must be an identifier
                match lhs.data {
                    TypeExpressionData::Identifier(identifier) => {
                        TypeExpressionData::VariantAccess(TypeVariantAccess {
                            name: identifier,
                            variant: rhs,
                            base: None,
                        })
                        .with_span(span)
                    }
                    _ => self.collect_error_and_continue_with_type_expression(
                        SpannedParserError {
                            error: ParserError::InvalidTypeVariantAccess,
                            span: op.span.clone(),
                        },
                    )?,
                }
            }

            // invalid operator
            _ => {
                return Err(SpannedParserError {
                    error: ParserError::UnexpectedToken {
                        expected: vec![Token::Plus, Token::Slash, Token::Dot],
                        found: op.token.clone(),
                    },
                    span: op.span.clone(),
                });
            }
        })
    }

    fn parse_type_prefix(
        &mut self,
    ) -> Result<TypeExpression, SpannedParserError> {
        match self.peek()?.token {
            // unary operators
            Token::Minus => {
                let op = self.advance()?;
                let rhs = self.parse_type_expression(UNARY_BP)?;
                let span = op.span.start..rhs.span.end;

                Ok(match rhs.data {
                    // if the rhs is a literal integer/decimal, parse it as negative literal
                    TypeExpressionData::Integer(value) => {
                        TypeExpressionData::Integer(-value)
                    }
                    TypeExpressionData::Decimal(value) => {
                        TypeExpressionData::Decimal(-value)
                    }
                    TypeExpressionData::TypedInteger(value) => match -value {
                        Ok(neg_value) => {
                            TypeExpressionData::TypedInteger(neg_value)
                        }
                        Err(e) => {
                            return Err(SpannedParserError {
                                error: ParserError::NumberParseError(
                                    NumberParseError::OutOfRange,
                                ),
                                span: rhs.span.clone(),
                            });
                        }
                    },
                    TypeExpressionData::TypedDecimal(value) => {
                        TypeExpressionData::TypedDecimal(-value)
                    }
                    // otherwise not a valid unary operation
                    _ => {
                        return Err(SpannedParserError {
                            error: ParserError::InvalidUnaryOperation {
                                operator: UnaryOperator::Arithmetic(
                                    ArithmeticUnaryOperator::Minus,
                                ),
                            },
                            span: op.span.clone(),
                        });
                    }
                }
                .with_span(span))
            }

            // ref (&)
            Token::Ampersand => {
                let op = self.advance()?;
                let rhs = self.parse_type_expression(UNARY_BP)?;
                let span = op.span.start..rhs.span.end;
                Ok(TypeExpressionData::Ref(Box::new(rhs)).with_span(span))
            }
            // mutable ref (&mut)
            Token::MutRef => {
                let op = self.advance()?;
                let rhs = self.parse_type_expression(UNARY_BP)?;
                let span = op.span.start..rhs.span.end;
                Ok(TypeExpressionData::RefMut(Box::new(rhs)).with_span(span))
            }

            // everything else is a value
            _ => self.parse_type_atom(),
        }
    }

    /// Returns the left and right binding powers for infix operators.
    /// The left binding power is used to determine if the operator can be parsed
    /// given the current minimum binding power, while the right binding power
    /// is used to determine the minimum binding power for the right-hand side expression.
    /// Left-associative operators have a binding power of (n, n+1),
    /// while right-associative operators have a binding power of (n, n).
    fn type_infix_binding_power(op: &Token) -> Option<(u8, u8)> {
        match op {
            // intersection type operator
            Token::Ampersand => Some((1, 2)),
            // union type operator
            Token::Pipe => Some((3, 4)),
            // interface operator
            Token::Plus => Some((5, 6)),
            // variant operator
            Token::Slash => Some((12, 13)),
            // property access
            Token::Dot => Some((14, 15)),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::ast::spanned::Spanned;
    use crate::ast::type_expressions::TypeExpression;
    use crate::ast::type_expressions::{
        Intersection, TypeExpressionData, Union,
    };
    use crate::parser::lexer::get_spanned_tokens_from_source;
    use crate::parser::{Parser, ParserOptions};

    pub fn parse_type_expression(src: &str) -> TypeExpression {
        let (tokens, errors) = get_spanned_tokens_from_source(src);
        if !errors.is_empty() {
            panic!("Lexer errors: {:?}", errors);
        }
        let mut parser =
            Parser::new_from_tokens(tokens, None, ParserOptions::default());
        parser.parse_type_expression(0).unwrap()
    }

    #[test]
    fn parse_simple_intersection_expression() {
        let expr = parse_type_expression("a & b");
        assert_eq!(
            expr.data,
            TypeExpressionData::Intersection(Intersection(vec![
                TypeExpressionData::Identifier("a".to_string())
                    .with_default_span(),
                TypeExpressionData::Identifier("b".to_string())
                    .with_default_span()
            ]))
        );
    }

    #[test]
    fn parse_simple_union_expression() {
        let expr = parse_type_expression("a | b");
        assert_eq!(
            expr.data,
            TypeExpressionData::Union(Union(vec![
                TypeExpressionData::Identifier("a".to_string())
                    .with_default_span(),
                TypeExpressionData::Identifier("b".to_string())
                    .with_default_span()
            ]))
        );
    }

    #[test]
    fn parse_variant_access_expression() {
        let expr = parse_type_expression("MyType/variant");
        assert_eq!(
            expr.data,
            TypeExpressionData::VariantAccess(
                crate::ast::type_expressions::TypeVariantAccess {
                    name: "MyType".to_string(),
                    variant: "variant".to_string(),
                    base: None,
                }
            )
        );
    }

    #[test]
    fn parse_ref_type_expression() {
        let expr = parse_type_expression("&MyType");
        assert_eq!(
            expr.data,
            TypeExpressionData::Ref(Box::new(
                TypeExpressionData::Identifier("MyType".to_string())
                    .with_default_span()
            ))
        );
    }

    #[test]
    fn parse_mut_ref_type_expression() {
        let expr = parse_type_expression("&mut MyType");
        assert_eq!(
            expr.data,
            TypeExpressionData::RefMut(Box::new(
                TypeExpressionData::Identifier("MyType".to_string())
                    .with_default_span()
            ))
        );
    }

    #[test]
    fn parse_multiple_ref_type_expression() {
        let expr = parse_type_expression("&mut &MyType");
        assert_eq!(
            expr.data,
            TypeExpressionData::RefMut(Box::new(
                TypeExpressionData::Ref(Box::new(
                    TypeExpressionData::Identifier("MyType".to_string())
                        .with_default_span()
                ))
                .with_default_span()
            ))
        );
    }

    #[test]
    fn parse_mut_keyword_variant_precedence() {
        let expr = parse_type_expression("&mut integer/u8");
        assert_eq!(
            expr.data,
            TypeExpressionData::RefMut(Box::new(
                TypeExpressionData::VariantAccess(
                    crate::ast::type_expressions::TypeVariantAccess {
                        name: "integer".to_string(),
                        variant: "u8".to_string(),
                        base: None,
                    }
                )
                .with_default_span()
            ))
        );
    }
}
