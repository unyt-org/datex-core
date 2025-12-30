use core::ops::Range;
use datex_core::ast::structs::expression::PropertyAccess;
use crate::ast::lexer::{SpannedToken, Token};
use crate::ast::spanned::Spanned;
use crate::ast::structs::expression::{Apply, BinaryOperation, DatexExpression, DatexExpressionData, RemoteExecution, UnaryOperation};
use crate::global::operators::binary::ArithmeticOperator;
use crate::global::operators::{ArithmeticUnaryOperator, BinaryOperator, UnaryOperator};
use crate::parser::errors::{ParserError, SpannedParserError};
use crate::parser::Parser;

static UNARY_BP: u8 = 5;

impl Parser {
    pub(crate) fn parse_expression(&mut self, min_bp: u8) -> Result<DatexExpression, SpannedParserError> {
        let mut lhs = self.parse_prefix()?;

        while self.has_more_tokens() {
            let (l_bp, r_bp) = match Parser::infix_binding_power(&self.peek()?.token) {
                Some(bp) if bp.0 >= min_bp => bp,
                _ => break,
            };

            let op = self.peek()?.clone();

            lhs = match op.token {
                // property access
                Token::Dot => {
                    self.advance()?; // consume the dot
                    let rhs = self.parse_expression(r_bp)?;
                    let span = lhs.span.start..rhs.span.end;

                    DatexExpressionData::PropertyAccess(PropertyAccess {
                        base: Box::new(lhs),
                        property: Box::new(rhs),
                    })
                    .with_span(span)
                }
                // binary operation
                Token::Plus | Token::Minus | Token::Star | Token::Slash => {
                    self.advance()?; // consume the operator
                    let rhs = self.parse_expression(r_bp)?;
                    let span = lhs.span.start..rhs.span.end;

                    DatexExpressionData::BinaryOperation(BinaryOperation {
                        left: Box::new(lhs),
                        operator: Parser::binary_operator_from_token(&op),
                        right: Box::new(rhs),
                        ty: None,
                    }).with_span(span)
                }
                // remote execution operator
                Token::DoubleColon => {
                    self.advance()?; // consume the operator
                    let rhs = self.parse_expression(r_bp)?;
                    let span = lhs.span.start..rhs.span.end;
                    DatexExpressionData::RemoteExecution(RemoteExecution {
                        left: Box::new(lhs),
                        right: Box::new(rhs),
                    }).with_span(span)
                }
                // apply
                _ => {
                    let (args, end) = self.parse_apply_arguments()?;
                    let span = lhs.span.start..end;

                    DatexExpressionData::Apply(Apply {
                        base: Box::new(lhs),
                        arguments: args,
                    }).with_span(span)
                }
            };
        }

        Ok(lhs)
    }

    // TODO: handle single value without parentheses as argument
    fn parse_apply_arguments(&mut self) -> Result<(Vec<DatexExpression>, usize), SpannedParserError> {

        let op = self.peek()?.clone();

        match op.token {
            // multiple arguments
            Token::LeftParen => {
                self.advance()?; // consume '('
                let mut args = Vec::new();

                while self.peek()?.token != Token::RightParen {
                    args.push(self.parse_expression(0)?);

                    if self.peek()?.token == Token::Comma {
                        self.advance()?;
                    }
                }

                let end = self.expect(Token::RightParen)?.span.end;
                Ok((args, end))
            }
            // single argument without parentheses
            _ => {
                let arg = self.parse_atom()?;
                let end = arg.span.end;
                Ok((vec![arg], end))
            }
        }

    }

    fn binary_operator_from_token(token: &SpannedToken) -> BinaryOperator {
        match token.token {
            Token::Plus => BinaryOperator::Arithmetic(ArithmeticOperator::Add),
            Token::Minus => BinaryOperator::Arithmetic(ArithmeticOperator::Subtract),
            Token::Star => BinaryOperator::Arithmetic(ArithmeticOperator::Multiply),
            Token::Slash => BinaryOperator::Arithmetic(ArithmeticOperator::Divide),
            _ => unreachable!(),
        }
    }

    fn unary_operator_from_token(token: &SpannedToken) -> Result<UnaryOperator, SpannedParserError> {
        match token.token {
            Token::Minus => Ok(UnaryOperator::Arithmetic(ArithmeticUnaryOperator::Minus)),
            _ => Err(SpannedParserError {
                error: ParserError::UnexpectedToken {
                    expected: vec![Token::Minus],
                    found: token.token.clone(),
                },
                span: token.span.clone(),
            }),
        }
    }

    fn parse_prefix(&mut self) -> Result<DatexExpression, SpannedParserError> {
        match self.peek()?.token {
            // unary operators
            Token::Minus => {
                let op = self.advance()?;
                let rhs = self.parse_expression(UNARY_BP)?;
                let span = op.span.start..rhs.span.end;
                Ok(DatexExpressionData::UnaryOperation(UnaryOperation {
                    operator: Parser::unary_operator_from_token(&op)?,
                    expression: Box::new(rhs),
                }).with_span(span))
            }

            // everything else is a value
            _ => self.parse_atom(),
        }
    }

    /// Returns the left and right binding powers for infix operators.
    /// The left binding power is used to determine if the operator can be parsed
    /// given the current minimum binding power, while the right binding power
    /// is used to determine the minimum binding power for the right-hand side expression.
    /// Left-associative operators have a binding power of (n, n+1),
    /// while right-associative operators have a binding power of (n, n).
    fn infix_binding_power(op: &Token) -> Option<(u8, u8)> {
        match op {
            // remote execution operator
            Token::DoubleColon => Some((1, 2)),
            // arithmetic operators
            Token::Plus | Token::Minus => Some((3, 4)),
            Token::Star | Token::Slash => Some((5, 6)),
            Token::Dot => Some((7, 8)),
            // apply (function call, type cast), which has same binding power as member access
            Token::LeftParen |
            Token::LeftCurly |
            Token::LeftBracket |
            Token::True |
            Token::False |
            Token::Null |
            Token::Identifier(_) |
            Token::StringLiteral(_) |
            Token::Infinity |
            Token::Nan |
            Token::DecimalNumericLiteral(_) |
            Token::HexadecimalIntegerLiteral(_) |
            Token::BinaryIntegerLiteral(_) |
            Token::PointerAddress(_) |
            Token::Endpoint(_)
            => Some((7, 8)),
            _ => None,
        }
    }
}


#[cfg(test)]
mod tests {
    use crate::ast::spanned::Spanned;
    use crate::ast::structs::expression::{Apply, BinaryOperation, DatexExpressionData, RemoteExecution, Statements, UnaryOperation};
    use crate::global::operators::{ArithmeticUnaryOperator, BinaryOperator, UnaryOperator};
    use crate::global::operators::binary::ArithmeticOperator;
    use crate::parser::tests::parse;

    #[test]
    fn parse_simple_binary_expression() {
        let expr = parse("true + false");
        assert_eq!(expr.data, DatexExpressionData::BinaryOperation(BinaryOperation {
            left: Box::new(DatexExpressionData::Boolean(true).with_default_span()),
            operator: BinaryOperator::Arithmetic(ArithmeticOperator::Add),
            right: Box::new(DatexExpressionData::Boolean(false).with_default_span()),
            ty: None,
        }));
    }

    #[test]
    fn parse_binary_expression_with_precedence() {
        let expr = parse("true + false * null");
        assert_eq!(expr.data, DatexExpressionData::BinaryOperation(BinaryOperation {
            left: Box::new(DatexExpressionData::Boolean(true).with_default_span()),
            operator: BinaryOperator::Arithmetic(ArithmeticOperator::Add),
            right: Box::new(DatexExpressionData::BinaryOperation(BinaryOperation {
                left: Box::new(DatexExpressionData::Boolean(false).with_default_span()),
                operator: BinaryOperator::Arithmetic(ArithmeticOperator::Multiply),
                right: Box::new(DatexExpressionData::Null.with_default_span()),
                ty: None,
            }).with_default_span()),
            ty: None,
        }));
    }

    #[test]
    fn parse_unary_expression() {
        let expr = parse("-true");
        assert_eq!(expr.data, DatexExpressionData::UnaryOperation(UnaryOperation {
            operator: UnaryOperator::Arithmetic(ArithmeticUnaryOperator::Minus),
            expression: Box::new(DatexExpressionData::Boolean(true).with_default_span()),
        }));
    }

    #[test]
    fn parse_property_access() {
        let expr = parse("myObject.myProperty");
        assert_eq!(expr.data, DatexExpressionData::PropertyAccess(crate::ast::structs::expression::PropertyAccess {
            base: Box::new(DatexExpressionData::Identifier("myObject".to_string()).with_default_span()),
            property: Box::new(DatexExpressionData::Identifier("myProperty".to_string()).with_default_span()),
        }));
    }

    #[test]
    fn parse_nested_property_access() {
        let expr = parse("myObject.innerObject.myProperty");
        assert_eq!(expr.data, DatexExpressionData::PropertyAccess(crate::ast::structs::expression::PropertyAccess {
            base: Box::new(DatexExpressionData::PropertyAccess(crate::ast::structs::expression::PropertyAccess {
                base: Box::new(DatexExpressionData::Identifier("myObject".to_string()).with_default_span()),
                property: Box::new(DatexExpressionData::Identifier("innerObject".to_string()).with_default_span()),
            }).with_default_span()),
            property: Box::new(DatexExpressionData::Identifier("myProperty".to_string()).with_default_span()),
        }));
    }

    #[test]
    fn parse_complex_expression() {
        let expr = parse("-myObject.value1 + myObject.value2 * true");
        assert_eq!(expr.data, DatexExpressionData::BinaryOperation(BinaryOperation {
            left: Box::new(DatexExpressionData::UnaryOperation(UnaryOperation {
                operator: UnaryOperator::Arithmetic(ArithmeticUnaryOperator::Minus),
                expression: Box::new(DatexExpressionData::PropertyAccess(crate::ast::structs::expression::PropertyAccess {
                    base: Box::new(DatexExpressionData::Identifier("myObject".to_string()).with_default_span()),
                    property: Box::new(DatexExpressionData::Identifier("value1".to_string()).with_default_span()),
                }).with_default_span()),
            }).with_default_span()),
            operator: BinaryOperator::Arithmetic(ArithmeticOperator::Add),
            right: Box::new(DatexExpressionData::BinaryOperation(BinaryOperation {
                left: Box::new(DatexExpressionData::PropertyAccess(crate::ast::structs::expression::PropertyAccess {
                    base: Box::new(DatexExpressionData::Identifier("myObject".to_string()).with_default_span()),
                    property: Box::new(DatexExpressionData::Identifier("value2".to_string()).with_default_span()),
                }).with_default_span()),
                operator: BinaryOperator::Arithmetic(ArithmeticOperator::Multiply),
                right: Box::new(DatexExpressionData::Boolean(true).with_default_span()),
                ty: None,
            }).with_default_span()),
            ty: None,
        }));
    }

    #[test]
    fn parse_apply() {
        let expr = parse("myFunction(arg1, arg2)");
        assert_eq!(expr.data, DatexExpressionData::Apply(crate::ast::structs::expression::Apply {
            base: Box::new(DatexExpressionData::Identifier("myFunction".to_string()).with_default_span()),
            arguments: vec![
                DatexExpressionData::Identifier("arg1".to_string()).with_default_span(),
                DatexExpressionData::Identifier("arg2".to_string()).with_default_span(),
            ],
        }));
    }

    #[test]
    fn parse_apply_without_parentheses() {
        let expr = parse("myFunction arg1");
        assert_eq!(expr.data, DatexExpressionData::Apply(crate::ast::structs::expression::Apply {
            base: Box::new(DatexExpressionData::Identifier("myFunction".to_string()).with_default_span()),
            arguments: vec![
                DatexExpressionData::Identifier("arg1".to_string()).with_default_span(),
            ],
        }));
    }

    #[test]
    fn parse_apply_with_property_access() {
        let expr = parse("myObject.myFunction(arg1)");
        assert_eq!(expr.data, DatexExpressionData::Apply(crate::ast::structs::expression::Apply {
            base: Box::new(DatexExpressionData::PropertyAccess(crate::ast::structs::expression::PropertyAccess {
                base: Box::new(DatexExpressionData::Identifier("myObject".to_string()).with_default_span()),
                property: Box::new(DatexExpressionData::Identifier("myFunction".to_string()).with_default_span()),
            }).with_default_span()),
            arguments: vec![
                DatexExpressionData::Identifier("arg1".to_string()).with_default_span(),
            ],
        }));
    }

    #[test]
    fn parse_remote_execution() {
        let expr = parse("endpoint::xy");
        assert_eq!(expr.data, DatexExpressionData::RemoteExecution(RemoteExecution {
            left: Box::new(DatexExpressionData::Identifier("endpoint".to_string()).with_default_span()),
            right: Box::new(DatexExpressionData::Identifier("xy".to_string()).with_default_span()),
        }));
    }

    #[test]
    fn parse_remote_execution_with_apply() {
        let expr = parse("endpoint::remoteFunction(arg1)");
        assert_eq!(expr.data, DatexExpressionData::RemoteExecution(RemoteExecution {
            left: Box::new(DatexExpressionData::Identifier("endpoint".to_string()).with_default_span()),
            right: Box::new(DatexExpressionData::Apply(Apply {
                base: Box::new(DatexExpressionData::Identifier("remoteFunction".to_string()).with_default_span()),
                arguments: vec![
                    DatexExpressionData::Identifier("arg1".to_string()).with_default_span(),
                ],
            }).with_default_span()),
        }));
    }

    #[test]
    fn parse_remote_execution_multiple_statements() {
        let expr = parse("endpoint::(statement1; statement2)");
        assert_eq!(expr.data, DatexExpressionData::RemoteExecution(RemoteExecution {
            left: Box::new(DatexExpressionData::Identifier("endpoint".to_string()).with_default_span()),
            right: Box::new(DatexExpressionData::Statements(Statements {
                statements: vec![
                    DatexExpressionData::Identifier("statement1".to_string()).with_default_span(),
                    DatexExpressionData::Identifier("statement2".to_string()).with_default_span(),
                ],
                is_terminated: false,
                unbounded: None,
            }).with_default_span()),
        }));
    }
}