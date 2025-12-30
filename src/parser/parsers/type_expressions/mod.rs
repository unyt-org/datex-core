mod atom;
mod list;
mod map;
mod key;
mod grouped;

use crate::ast::lexer::{SpannedToken, Token};
use crate::ast::spanned::Spanned;
use crate::ast::structs::r#type::{Intersection, TypeExpressionData, TypeExpression, Union};
use crate::parser::errors::{ParserError, SpannedParserError};
use crate::parser::Parser;

static UNARY_BP: u8 = 100;

impl Parser {
    pub(crate) fn parse_type_expression(&mut self, min_bp: u8) -> Result<TypeExpression, SpannedParserError> {
        let mut lhs = self.parse_type_prefix()?;

        while self.has_more_tokens() {
            let (_, r_bp) = match Parser::type_infix_binding_power(&self.peek()?.token) {
                Some(bp) if bp.0 >= min_bp => bp,
                _ => break,
            };

            let op = self.peek()?.clone();

            lhs = self.parse_type_binary_operation(lhs, op, r_bp)?;
        }

        Ok(lhs)
    }

    fn parse_type_binary_operation(&mut self, lhs: TypeExpression, op: SpannedToken, r_bp: u8) -> Result<TypeExpression, SpannedParserError> {
        Ok(match op.token {

            // union type operator
            Token::Pipe => {
                self.advance()?; // consume operator
                let rhs = self.parse_type_expression(r_bp)?;
                let span = lhs.span.start..rhs.span.end;
                TypeExpressionData::Union(Union(vec![lhs, rhs]))
                    .with_span(span)
            }
            // intersection type operator
            Token::Ampersand => {
                self.advance()?; // consume operator
                let rhs = self.parse_type_expression(r_bp)?;
                let span = lhs.span.start..rhs.span.end;
                TypeExpressionData::Intersection(Intersection(vec![lhs, rhs]))
                    .with_span(span)
            }

            // invalid operator
            _ => return Err(SpannedParserError {
                error: ParserError::UnexpectedToken {
                    expected: vec![
                        Token::Plus,
                        Token::Slash,
                        Token::Dot,
                    ],
                    found: op.token.clone(),
                },
                span: op.span.clone(),
            }),
        })
    }


    fn parse_type_prefix(&mut self) -> Result<TypeExpression, SpannedParserError> {
        match self.peek()?.token {
            // unary operators
            Token::Minus => {
                todo!()
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
            Token::Slash => Some((7, 8)),
            // property access
            Token::Dot => Some((9, 10)),
            _ => None,
        }
    }
}


#[cfg(test)]
mod tests {
    use datex_core::ast::structs::r#type::TypeExpression;
    use crate::ast::lexer::get_spanned_tokens_from_source;
    use crate::ast::spanned::Spanned;
    use crate::ast::structs::r#type::{TypeExpressionData, Intersection};
    use crate::parser::Parser;
    use crate::parser::tests::parse;


    pub fn parse_type_expression(src: &str) -> TypeExpression {
        let tokens = get_spanned_tokens_from_source(src).unwrap();
        let mut parser = Parser::new(tokens);
        parser.parse_type_expression(0).unwrap()
    }

    #[test]
    fn parse_simple_union_expression() {
        let expr = parse_type_expression("a & b");
        assert_eq!(expr.data, TypeExpressionData::Intersection(Intersection(
            vec![
                TypeExpressionData::Identifier("a".to_string()).with_default_span(),
                TypeExpressionData::Identifier("b".to_string()).with_default_span()
            ]
        )));
    }

}