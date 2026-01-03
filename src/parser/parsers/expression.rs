use crate::ast::expressions::PropertyAccess;
use crate::ast::expressions::{
    Apply, BinaryOperation, ComparisonOperation, CreateRef, DatexExpression,
    DatexExpressionData, Deref, DerefAssignment, GenericInstantiation,
    PropertyAssignment, RemoteExecution, SlotAssignment, UnaryOperation,
    VariableAssignment,
};
use crate::ast::spanned::Spanned;
use crate::global::operators::binary::{
    ArithmeticOperator, BitwiseOperator, LogicalOperator,
};
use crate::global::operators::{
    ArithmeticUnaryOperator, AssignmentOperator, BinaryOperator,
    ComparisonOperator, LogicalUnaryOperator, UnaryOperator,
};
use crate::parser::Parser;
use crate::parser::errors::{ParserError, SpannedParserError};
use crate::parser::lexer::{SpannedToken, Token};
use crate::references::reference::ReferenceMutability;
use crate::values::core_values::error::NumberParseError;

static UNARY_BP: u8 = 22; // weaker than property access / apply, stronger than all other binary operators

impl Parser {
    pub(crate) fn parse_expression(
        &mut self,
        min_bp: u8,
    ) -> Result<DatexExpression, SpannedParserError> {
        let mut lhs = self.parse_prefix()?;

        while self.has_more_tokens() {
            let (_, r_bp) =
                match Parser::infix_binding_power(&self.peek()?.token) {
                    Some(bp) if bp.0 >= min_bp => bp,
                    _ => break,
                };

            let op = self.peek()?.clone();

            lhs = self.parse_binary_operation(lhs, op, r_bp)?;
        }

        Ok(lhs)
    }

    fn parse_binary_operation(
        &mut self,
        lhs: DatexExpression,
        op: SpannedToken,
        r_bp: u8,
    ) -> Result<DatexExpression, SpannedParserError> {
        Ok(match op.token {
            // property access
            Token::Dot => {
                self.advance()?; // consume the dot

                let rhs = self.parse_key()?;

                let span = lhs.span.start..rhs.span.end;

                DatexExpressionData::PropertyAccess(PropertyAccess {
                    base: Box::new(lhs),
                    property: Box::new(rhs),
                })
                .with_span(span)
            }
            // binary operations
            Token::Plus
            | Token::Minus
            | Token::Star
            | Token::Slash
            | Token::Caret
            | Token::And
            | Token::Or
            | Token::Ampersand
            | Token::Pipe => {
                self.advance()?; // consume the operator
                let rhs = self.parse_expression(r_bp)?;
                let span = lhs.span.start..rhs.span.end;

                DatexExpressionData::BinaryOperation(BinaryOperation {
                    left: Box::new(lhs),
                    operator: Parser::binary_operator_from_token(&op),
                    right: Box::new(rhs),
                    ty: None,
                })
                .with_span(span)
            }

            // comparison operators
            Token::Equal
            | Token::Is
            | Token::StructuralEqual
            | Token::NotEqual
            | Token::NotStructuralEqual
            | Token::LessEqual
            | Token::RightAngle
            | Token::GreaterEqual => {
                self.advance()?; // consume the operator
                let rhs = self.parse_expression(r_bp)?;
                let span = lhs.span.start..rhs.span.end;

                DatexExpressionData::ComparisonOperation(ComparisonOperation {
                    left: Box::new(lhs),
                    operator: Parser::comparison_operator_from_token(&op),
                    right: Box::new(rhs),
                })
                .with_span(span)
            }

            // generic parameters or fall back to less than operator if not generic parameters
            Token::LeftAngle => {
                let generic_params =
                    self.try_parse_generic_parameters_or_roll_back();
                match generic_params {
                    Ok((params, end_span)) => {
                        let span = lhs.span.start..end_span.end;
                        DatexExpressionData::GenericInstantiation(
                            GenericInstantiation {
                                base: Box::new(lhs),
                                generic_arguments: params,
                            },
                        )
                        .with_span(span)
                    }
                    _ => {
                        self.advance()?; // consume the operator
                        let rhs = self.parse_expression(r_bp)?;
                        let span = lhs.span.start..rhs.span.end;

                        DatexExpressionData::ComparisonOperation(
                            ComparisonOperation {
                                left: Box::new(lhs),
                                operator: ComparisonOperator::LessThan,
                                right: Box::new(rhs),
                            },
                        )
                        .with_span(span)
                    }
                }
            }

            // assignment (=)
            Token::Assign
            | Token::AddAssign
            | Token::SubAssign
            | Token::MulAssign
            | Token::DivAssign => self.parse_assignment(lhs, op, r_bp)?,

            // remote execution operator
            Token::DoubleColon => {
                self.advance()?; // consume the operator
                let rhs = self.parse_expression(r_bp)?;
                let span = lhs.span.start..rhs.span.end;
                DatexExpressionData::RemoteExecution(RemoteExecution {
                    left: Box::new(lhs),
                    right: Box::new(rhs),
                })
                .with_span(span)
            }
            // apply
            _ => {
                let (args, end) = self.parse_apply_arguments()?;
                let span = lhs.span.start..end;

                DatexExpressionData::Apply(Apply {
                    base: Box::new(lhs),
                    arguments: args,
                })
                .with_span(span)
            }
        })
    }

    fn parse_assignment(
        &mut self,
        lhs: DatexExpression,
        op: SpannedToken,
        r_bp: u8,
    ) -> Result<DatexExpression, SpannedParserError> {
        self.advance()?; // consume the operator
        let rhs = self.parse_expression(r_bp)?;
        let span = lhs.span.start..rhs.span.end;
        let assignment_operator = match op.token {
            Token::Assign => AssignmentOperator::Assign,
            Token::AddAssign => AssignmentOperator::AddAssign,
            Token::SubAssign => AssignmentOperator::SubtractAssign,
            Token::MulAssign => AssignmentOperator::MultiplyAssign,
            Token::DivAssign => AssignmentOperator::DivideAssign,
            _ => unreachable!(),
        };

        // select assignment type based on lhs
        Ok(match lhs.data {
            // variable assignment
            DatexExpressionData::Identifier(name) => {
                DatexExpressionData::VariableAssignment(VariableAssignment {
                    id: None,
                    name,
                    operator: assignment_operator,
                    expression: Box::new(rhs),
                })
            }
            // property assignment
            DatexExpressionData::PropertyAccess(prop_access) => {
                DatexExpressionData::PropertyAssignment(PropertyAssignment {
                    operator: assignment_operator,
                    base: prop_access.base,
                    property: prop_access.property,
                    assigned_expression: Box::new(rhs),
                })
            }
            // deref assignment
            DatexExpressionData::Deref(deref) => {
                DatexExpressionData::DerefAssignment(DerefAssignment {
                    operator: assignment_operator,
                    deref_expression: deref.expression,
                    assigned_expression: Box::new(rhs),
                })
            }
            // slot assignment
            DatexExpressionData::Slot(slot) => {
                DatexExpressionData::SlotAssignment(SlotAssignment {
                    slot,
                    expression: Box::new(rhs),
                })
            }
            // invalid lhs for assignment
            _ => {
                return self.collect_error_and_continue(SpannedParserError {
                    error: ParserError::InvalidAssignmentTarget,
                    span: lhs.span.clone(),
                });
            }
        }
        .with_span(span))
    }

    // TODO: handle single value without parentheses as argument
    fn parse_apply_arguments(
        &mut self,
    ) -> Result<(Vec<DatexExpression>, usize), SpannedParserError> {
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
            Token::Minus => {
                BinaryOperator::Arithmetic(ArithmeticOperator::Subtract)
            }
            Token::Star => {
                BinaryOperator::Arithmetic(ArithmeticOperator::Multiply)
            }
            Token::Slash => {
                BinaryOperator::Arithmetic(ArithmeticOperator::Divide)
            }
            Token::Caret => {
                BinaryOperator::Arithmetic(ArithmeticOperator::Power)
            }
            Token::And => BinaryOperator::Logical(LogicalOperator::And),
            Token::Or => BinaryOperator::Logical(LogicalOperator::Or),
            Token::Ampersand => BinaryOperator::Bitwise(BitwiseOperator::And),
            Token::Pipe => BinaryOperator::Bitwise(BitwiseOperator::Or),
            _ => unreachable!(),
        }
    }

    fn comparison_operator_from_token(
        token: &SpannedToken,
    ) -> ComparisonOperator {
        match token.token {
            Token::Is => ComparisonOperator::Is,
            Token::Equal => ComparisonOperator::Equal,
            Token::StructuralEqual => ComparisonOperator::StructuralEqual,
            Token::NotEqual => ComparisonOperator::NotEqual,
            Token::NotStructuralEqual => ComparisonOperator::NotStructuralEqual,
            Token::LeftAngle => ComparisonOperator::LessThan,
            Token::LessEqual => ComparisonOperator::LessThanOrEqual,
            Token::RightAngle => ComparisonOperator::GreaterThan,
            Token::GreaterEqual => ComparisonOperator::GreaterThanOrEqual,
            _ => unreachable!(),
        }
    }

    fn unary_operator_from_token(
        token: &SpannedToken,
    ) -> Result<UnaryOperator, SpannedParserError> {
        match token.token {
            Token::Minus => {
                Ok(UnaryOperator::Arithmetic(ArithmeticUnaryOperator::Minus))
            }
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
            // minus:
            Token::Minus => {
                let op = self.advance()?;
                let rhs = self.parse_expression(UNARY_BP)?;
                let span = op.span.start..rhs.span.end;

                Ok(match rhs.data {
                    // special case: if the rhs is a literal integer/decimal, parse it as negative literal
                    DatexExpressionData::Integer(value) => {
                        DatexExpressionData::Integer(-value)
                    }
                    DatexExpressionData::Decimal(value) => {
                        DatexExpressionData::Decimal(-value)
                    }
                    DatexExpressionData::TypedInteger(value) => match -value {
                        Ok(neg_value) => {
                            DatexExpressionData::TypedInteger(neg_value)
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
                    DatexExpressionData::TypedDecimal(value) => {
                        DatexExpressionData::TypedDecimal(-value)
                    }
                    // default case: unary minus operation
                    _ => DatexExpressionData::UnaryOperation(UnaryOperation {
                        operator: UnaryOperator::Arithmetic(
                            ArithmeticUnaryOperator::Minus,
                        ),
                        expression: Box::new(rhs),
                    }),
                }
                .with_span(span))
            }
            // plus
            Token::Plus => {
                let op = self.advance()?;
                let rhs = self.parse_expression(UNARY_BP)?;
                let span = op.span.start..rhs.span.end;
                Ok(match rhs.data {
                    // special case: unary plus is a no-op for integer and decimal literals
                    DatexExpressionData::Integer(value) => {
                        DatexExpressionData::Integer(value)
                    }
                    DatexExpressionData::Decimal(value) => {
                        DatexExpressionData::Decimal(value)
                    }
                    DatexExpressionData::TypedInteger(value) => {
                        DatexExpressionData::TypedInteger(value)
                    }
                    DatexExpressionData::TypedDecimal(value) => {
                        DatexExpressionData::TypedDecimal(value)
                    }
                    // default case: unary plus operation
                    _ => DatexExpressionData::UnaryOperation(UnaryOperation {
                        operator: UnaryOperator::Arithmetic(
                            ArithmeticUnaryOperator::Plus,
                        ),
                        expression: Box::new(rhs),
                    }),
                }
                .with_span(span))
            }
            // negation (!)
            Token::Exclamation => {
                let op = self.advance()?;
                let rhs = self.parse_expression(UNARY_BP)?;
                let span = op.span.start..rhs.span.end;
                Ok(DatexExpressionData::UnaryOperation(UnaryOperation {
                    operator: UnaryOperator::Logical(LogicalUnaryOperator::Not),
                    expression: Box::new(rhs),
                })
                .with_span(span))
            }
            // ref (&)
            Token::Ampersand => {
                let op = self.advance()?;
                let rhs = self.parse_expression(UNARY_BP)?;
                let span = op.span.start..rhs.span.end;
                Ok(DatexExpressionData::CreateRef(CreateRef {
                    mutability: ReferenceMutability::Immutable,
                    expression: Box::new(rhs),
                })
                .with_span(span))
            }
            // mutable ref (&mut)
            Token::MutRef => {
                let op = self.advance()?;
                let rhs = self.parse_expression(UNARY_BP)?;
                let span = op.span.start..rhs.span.end;
                Ok(DatexExpressionData::CreateRef(CreateRef {
                    mutability: ReferenceMutability::Mutable,
                    expression: Box::new(rhs),
                })
                .with_span(span))
            }
            // deref (*)
            Token::Star => {
                let op = self.advance()?;
                let rhs = self.parse_expression(UNARY_BP)?;
                let span = op.span.start..rhs.span.end;
                Ok(DatexExpressionData::Deref(Deref {
                    expression: Box::new(rhs),
                })
                .with_span(span))
            }

            // everything else is an atom
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
            // assignment operators
            Token::Assign
            | Token::AddAssign
            | Token::SubAssign
            | Token::MulAssign
            | Token::DivAssign => Some((3, 3)),
            // comparison operators
            Token::Equal
            | Token::NotEqual
            | Token::StructuralEqual
            | Token::NotStructuralEqual
            | Token::Is => Some((5, 6)),
            Token::LeftAngle
            | Token::LessEqual
            | Token::RightAngle
            | Token::GreaterEqual => Some((7, 8)),
            // logical operators
            Token::Or => Some((9, 10)),
            Token::And => Some((11, 12)),
            // arithmetic operators
            Token::Plus | Token::Minus => Some((13, 14)),
            Token::Star | Token::Slash => Some((15, 16)),
            Token::Caret => Some((17, 17)), // right associative
            // bitwise operators
            Token::Pipe => Some((18, 19)),
            Token::Ampersand => Some((20, 21)),
            // property access
            Token::Dot => Some((23, 24)),
            // apply (function call, type cast), which has same binding power as member access
            Token::LeftParen
            | Token::LeftCurly
            | Token::LeftBracket
            | Token::True
            | Token::False
            | Token::Null
            | Token::Identifier(_)
            | Token::StringLiteral(_)
            | Token::Infinity
            | Token::Nan
            | Token::HexadecimalIntegerLiteral(_)
            | Token::BinaryIntegerLiteral(_)
            | Token::OctalIntegerLiteral(_)
            | Token::IntegerLiteral(_)
            | Token::DecimalLiteral(_)
            | Token::PointerAddress(_)
            | Token::Slot(_)
            | Token::PointerAddress(_)
            | Token::Endpoint(_) => Some((23, 24)),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::ast::expressions::{
        Apply, BinaryOperation, ComparisonOperation, CreateRef,
        DatexExpressionData, Deref, DerefAssignment, GenericInstantiation,
        PropertyAccess, PropertyAssignment, RemoteExecution, Slot,
        SlotAssignment, Statements, UnaryOperation, VariableAssignment,
    };
    use crate::ast::spanned::Spanned;
    use crate::ast::type_expressions::TypeExpressionData;
    use crate::global::operators::binary::{
        ArithmeticOperator, BitwiseOperator, LogicalOperator,
    };
    use crate::global::operators::{
        ArithmeticUnaryOperator, AssignmentOperator, BinaryOperator,
        ComparisonOperator, LogicalUnaryOperator, UnaryOperator,
    };
    use crate::parser::errors::ParserError;
    use crate::parser::tests::{parse, try_parse_and_return_on_first_error};
    use crate::references::reference::ReferenceMutability;

    #[test]
    fn parse_simple_binary_expression() {
        let expr = parse("true + false");
        assert_eq!(
            expr.data,
            DatexExpressionData::BinaryOperation(BinaryOperation {
                left: Box::new(
                    DatexExpressionData::Boolean(true).with_default_span()
                ),
                operator: BinaryOperator::Arithmetic(ArithmeticOperator::Add),
                right: Box::new(
                    DatexExpressionData::Boolean(false).with_default_span()
                ),
                ty: None,
            })
        );
    }

    #[test]
    fn parse_binary_expression_with_precedence() {
        let expr = parse("true + false * null");
        assert_eq!(
            expr.data,
            DatexExpressionData::BinaryOperation(BinaryOperation {
                left: Box::new(
                    DatexExpressionData::Boolean(true).with_default_span()
                ),
                operator: BinaryOperator::Arithmetic(ArithmeticOperator::Add),
                right: Box::new(
                    DatexExpressionData::BinaryOperation(BinaryOperation {
                        left: Box::new(
                            DatexExpressionData::Boolean(false)
                                .with_default_span()
                        ),
                        operator: BinaryOperator::Arithmetic(
                            ArithmeticOperator::Multiply
                        ),
                        right: Box::new(
                            DatexExpressionData::Null.with_default_span()
                        ),
                        ty: None,
                    })
                    .with_default_span()
                ),
                ty: None,
            })
        );
    }

    #[test]
    fn parse_unary_expression() {
        let expr = parse("-true");
        assert_eq!(
            expr.data,
            DatexExpressionData::UnaryOperation(UnaryOperation {
                operator: UnaryOperator::Arithmetic(
                    ArithmeticUnaryOperator::Minus
                ),
                expression: Box::new(
                    DatexExpressionData::Boolean(true).with_default_span()
                ),
            })
        );
    }

    #[test]
    fn parse_property_access() {
        let expr = parse("myObject.myProperty");
        assert_eq!(
            expr.data,
            DatexExpressionData::PropertyAccess(PropertyAccess {
                base: Box::new(
                    DatexExpressionData::Identifier("myObject".to_string())
                        .with_default_span()
                ),
                property: Box::new(
                    DatexExpressionData::Text("myProperty".to_string())
                        .with_default_span()
                ),
            })
        );
    }

    #[test]
    fn parse_property_access_reserved_keywords() {
        let expr = parse("myObject.if");
        assert_eq!(
            expr.data,
            DatexExpressionData::PropertyAccess(PropertyAccess {
                base: Box::new(
                    DatexExpressionData::Identifier("myObject".to_string())
                        .with_default_span()
                ),
                property: Box::new(
                    DatexExpressionData::Text("if".to_string())
                        .with_default_span()
                ),
            })
        );
    }

    #[test]
    fn parse_nested_property_access() {
        let expr = parse("myObject.innerObject.myProperty");
        assert_eq!(
            expr.data,
            DatexExpressionData::PropertyAccess(PropertyAccess {
                base: Box::new(
                    DatexExpressionData::PropertyAccess(PropertyAccess {
                        base: Box::new(
                            DatexExpressionData::Identifier(
                                "myObject".to_string()
                            )
                            .with_default_span()
                        ),
                        property: Box::new(
                            DatexExpressionData::Text(
                                "innerObject".to_string()
                            )
                            .with_default_span()
                        ),
                    })
                    .with_default_span()
                ),
                property: Box::new(
                    DatexExpressionData::Text("myProperty".to_string())
                        .with_default_span()
                ),
            })
        );
    }

    #[test]
    fn parse_complex_expression() {
        let expr = parse("-myObject.value1 + myObject.value2 * true");
        assert_eq!(
            expr.data,
            DatexExpressionData::BinaryOperation(BinaryOperation {
                left: Box::new(
                    DatexExpressionData::UnaryOperation(UnaryOperation {
                        operator: UnaryOperator::Arithmetic(
                            ArithmeticUnaryOperator::Minus
                        ),
                        expression: Box::new(
                            DatexExpressionData::PropertyAccess(
                                PropertyAccess {
                                    base: Box::new(
                                        DatexExpressionData::Identifier(
                                            "myObject".to_string()
                                        )
                                        .with_default_span()
                                    ),
                                    property: Box::new(
                                        DatexExpressionData::Text(
                                            "value1".to_string()
                                        )
                                        .with_default_span()
                                    ),
                                }
                            )
                            .with_default_span()
                        ),
                    })
                    .with_default_span()
                ),
                operator: BinaryOperator::Arithmetic(ArithmeticOperator::Add),
                right: Box::new(
                    DatexExpressionData::BinaryOperation(BinaryOperation {
                        left: Box::new(
                            DatexExpressionData::PropertyAccess(
                                PropertyAccess {
                                    base: Box::new(
                                        DatexExpressionData::Identifier(
                                            "myObject".to_string()
                                        )
                                        .with_default_span()
                                    ),
                                    property: Box::new(
                                        DatexExpressionData::Text(
                                            "value2".to_string()
                                        )
                                        .with_default_span()
                                    ),
                                }
                            )
                            .with_default_span()
                        ),
                        operator: BinaryOperator::Arithmetic(
                            ArithmeticOperator::Multiply
                        ),
                        right: Box::new(
                            DatexExpressionData::Boolean(true)
                                .with_default_span()
                        ),
                        ty: None,
                    })
                    .with_default_span()
                ),
                ty: None,
            })
        );
    }

    #[test]
    fn parse_apply() {
        let expr = parse("myFunction(arg1, arg2)");
        assert_eq!(
            expr.data,
            DatexExpressionData::Apply(Apply {
                base: Box::new(
                    DatexExpressionData::Identifier("myFunction".to_string())
                        .with_default_span()
                ),
                arguments: vec![
                    DatexExpressionData::Identifier("arg1".to_string())
                        .with_default_span(),
                    DatexExpressionData::Identifier("arg2".to_string())
                        .with_default_span(),
                ],
            })
        );
    }

    #[test]
    fn parse_apply_empty_arguments() {
        let expr = parse("myFunction()");
        assert_eq!(
            expr.data,
            DatexExpressionData::Apply(Apply {
                base: Box::new(
                    DatexExpressionData::Identifier("myFunction".to_string())
                        .with_default_span()
                ),
                arguments: vec![],
            })
        );
    }

    #[test]
    fn parse_apply_without_parentheses() {
        let expr = parse("myFunction arg1");
        assert_eq!(
            expr.data,
            DatexExpressionData::Apply(Apply {
                base: Box::new(
                    DatexExpressionData::Identifier("myFunction".to_string())
                        .with_default_span()
                ),
                arguments: vec![
                    DatexExpressionData::Identifier("arg1".to_string())
                        .with_default_span(),
                ],
            })
        );
    }

    #[test]
    fn parse_apply_with_property_access() {
        let expr = parse("myObject.myFunction(arg1)");
        assert_eq!(
            expr.data,
            DatexExpressionData::Apply(Apply {
                base: Box::new(
                    DatexExpressionData::PropertyAccess(PropertyAccess {
                        base: Box::new(
                            DatexExpressionData::Identifier(
                                "myObject".to_string()
                            )
                            .with_default_span()
                        ),
                        property: Box::new(
                            DatexExpressionData::Text("myFunction".to_string())
                                .with_default_span()
                        ),
                    })
                    .with_default_span()
                ),
                arguments: vec![
                    DatexExpressionData::Identifier("arg1".to_string())
                        .with_default_span(),
                ],
            })
        );
    }

    #[test]
    fn parse_multiple_applies() {
        let expr = parse("myFunction(arg1)(arg2)");
        assert_eq!(
            expr.data,
            DatexExpressionData::Apply(Apply {
                base: Box::new(
                    DatexExpressionData::Apply(Apply {
                        base: Box::new(
                            DatexExpressionData::Identifier(
                                "myFunction".to_string()
                            )
                            .with_default_span()
                        ),
                        arguments: vec![
                            DatexExpressionData::Identifier("arg1".to_string())
                                .with_default_span(),
                        ],
                    })
                    .with_default_span()
                ),
                arguments: vec![
                    DatexExpressionData::Identifier("arg2".to_string())
                        .with_default_span(),
                ],
            })
        );
    }

    #[test]
    fn parse_remote_execution() {
        let expr = parse("endpoint::xy");
        assert_eq!(
            expr.data,
            DatexExpressionData::RemoteExecution(RemoteExecution {
                left: Box::new(
                    DatexExpressionData::Identifier("endpoint".to_string())
                        .with_default_span()
                ),
                right: Box::new(
                    DatexExpressionData::Identifier("xy".to_string())
                        .with_default_span()
                ),
            })
        );
    }

    #[test]
    fn parse_remote_execution_with_apply() {
        let expr = parse("endpoint::remoteFunction(arg1)");
        assert_eq!(
            expr.data,
            DatexExpressionData::RemoteExecution(RemoteExecution {
                left: Box::new(
                    DatexExpressionData::Identifier("endpoint".to_string())
                        .with_default_span()
                ),
                right: Box::new(
                    DatexExpressionData::Apply(Apply {
                        base: Box::new(
                            DatexExpressionData::Identifier(
                                "remoteFunction".to_string()
                            )
                            .with_default_span()
                        ),
                        arguments: vec![
                            DatexExpressionData::Identifier("arg1".to_string())
                                .with_default_span(),
                        ],
                    })
                    .with_default_span()
                ),
            })
        );
    }

    #[test]
    fn parse_remote_execution_multiple_statements() {
        let expr = parse("endpoint::(statement1; statement2)");
        assert_eq!(
            expr.data,
            DatexExpressionData::RemoteExecution(RemoteExecution {
                left: Box::new(
                    DatexExpressionData::Identifier("endpoint".to_string())
                        .with_default_span()
                ),
                right: Box::new(
                    DatexExpressionData::Statements(Statements {
                        statements: vec![
                            DatexExpressionData::Identifier(
                                "statement1".to_string()
                            )
                            .with_default_span(),
                            DatexExpressionData::Identifier(
                                "statement2".to_string()
                            )
                            .with_default_span(),
                        ],
                        is_terminated: false,
                        unbounded: None,
                    })
                    .with_default_span()
                ),
            })
        );
    }

    #[test]
    fn parse_and() {
        let expr = parse("true and false");
        assert_eq!(
            expr.data,
            DatexExpressionData::BinaryOperation(BinaryOperation {
                left: Box::new(
                    DatexExpressionData::Boolean(true).with_default_span()
                ),
                operator: BinaryOperator::Logical(LogicalOperator::And),
                right: Box::new(
                    DatexExpressionData::Boolean(false).with_default_span()
                ),
                ty: None,
            })
        );
    }

    #[test]
    fn parse_or() {
        let expr = parse("true or false");
        assert_eq!(
            expr.data,
            DatexExpressionData::BinaryOperation(BinaryOperation {
                left: Box::new(
                    DatexExpressionData::Boolean(true).with_default_span()
                ),
                operator: BinaryOperator::Logical(LogicalOperator::Or),
                right: Box::new(
                    DatexExpressionData::Boolean(false).with_default_span()
                ),
                ty: None,
            })
        );
    }

    #[test]
    fn parse_bitwise_and() {
        let expr = parse("1 & 2");
        assert_eq!(
            expr.data,
            DatexExpressionData::BinaryOperation(BinaryOperation {
                left: Box::new(
                    DatexExpressionData::Integer(1.into()).with_default_span()
                ),
                operator: BinaryOperator::Bitwise(BitwiseOperator::And),
                right: Box::new(
                    DatexExpressionData::Integer(2.into()).with_default_span()
                ),
                ty: None,
            })
        );
    }

    #[test]
    fn parse_bitwise_or() {
        let expr = parse("1 | 2");
        assert_eq!(
            expr.data,
            DatexExpressionData::BinaryOperation(BinaryOperation {
                left: Box::new(
                    DatexExpressionData::Integer(1.into()).with_default_span()
                ),
                operator: BinaryOperator::Bitwise(BitwiseOperator::Or),
                right: Box::new(
                    DatexExpressionData::Integer(2.into()).with_default_span()
                ),
                ty: None,
            })
        );
    }

    #[test]
    fn parse_logical_expression_precedence() {
        let expr = parse("true or false and true");
        assert_eq!(
            expr.data,
            DatexExpressionData::BinaryOperation(BinaryOperation {
                left: Box::new(
                    DatexExpressionData::Boolean(true).with_default_span()
                ),
                operator: BinaryOperator::Logical(LogicalOperator::Or),
                right: Box::new(
                    DatexExpressionData::BinaryOperation(BinaryOperation {
                        left: Box::new(
                            DatexExpressionData::Boolean(false)
                                .with_default_span()
                        ),
                        operator: BinaryOperator::Logical(LogicalOperator::And),
                        right: Box::new(
                            DatexExpressionData::Boolean(true)
                                .with_default_span()
                        ),
                        ty: None,
                    })
                    .with_default_span()
                ),
                ty: None,
            })
        );
    }

    #[test]
    fn parse_immutable_reference() {
        let expr = parse("&myVar");
        assert_eq!(
            expr.data,
            DatexExpressionData::CreateRef(CreateRef {
                mutability: ReferenceMutability::Immutable,
                expression: Box::new(
                    DatexExpressionData::Identifier("myVar".to_string())
                        .with_default_span()
                ),
            })
        );
    }

    #[test]
    fn parse_mutable_reference() {
        let expr = parse("&mut myVar");
        assert_eq!(
            expr.data,
            DatexExpressionData::CreateRef(CreateRef {
                mutability: ReferenceMutability::Mutable,
                expression: Box::new(
                    DatexExpressionData::Identifier("myVar".to_string())
                        .with_default_span()
                ),
            })
        );
    }

    #[test]
    fn parse_ref_of_property_access() {
        let expr = parse("&myObject.myProperty");
        assert_eq!(
            expr.data,
            DatexExpressionData::CreateRef(CreateRef {
                mutability: ReferenceMutability::Immutable,
                expression: Box::new(
                    DatexExpressionData::PropertyAccess(PropertyAccess {
                        base: Box::new(
                            DatexExpressionData::Identifier(
                                "myObject".to_string()
                            )
                            .with_default_span()
                        ),
                        property: Box::new(
                            DatexExpressionData::Text("myProperty".to_string())
                                .with_default_span()
                        ),
                    })
                    .with_default_span()
                ),
            })
        );
    }

    #[test]
    fn parse_multiple_refs() {
        let expr = parse("&mut &myVar");
        assert_eq!(
            expr.data,
            DatexExpressionData::CreateRef(CreateRef {
                mutability: ReferenceMutability::Mutable,
                expression: Box::new(
                    DatexExpressionData::CreateRef(CreateRef {
                        mutability: ReferenceMutability::Immutable,
                        expression: Box::new(
                            DatexExpressionData::Identifier(
                                "myVar".to_string()
                            )
                            .with_default_span()
                        ),
                    })
                    .with_default_span()
                ),
            })
        );
    }

    #[test]
    fn parse_dereference() {
        let expr = parse("*myRef");
        assert_eq!(
            expr.data,
            DatexExpressionData::Deref(Deref {
                expression: Box::new(
                    DatexExpressionData::Identifier("myRef".to_string())
                        .with_default_span()
                ),
            })
        );
    }

    #[test]
    fn parse_dereference_of_reference() {
        let expr = parse("*&myVar");
        assert_eq!(
            expr.data,
            DatexExpressionData::Deref(Deref {
                expression: Box::new(
                    DatexExpressionData::CreateRef(CreateRef {
                        mutability: ReferenceMutability::Immutable,
                        expression: Box::new(
                            DatexExpressionData::Identifier(
                                "myVar".to_string()
                            )
                            .with_default_span()
                        ),
                    })
                    .with_default_span()
                ),
            })
        );
    }

    #[test]
    fn parse_multiple_dereferences() {
        let expr = parse("**myRef");
        assert_eq!(
            expr.data,
            DatexExpressionData::Deref(Deref {
                expression: Box::new(
                    DatexExpressionData::Deref(Deref {
                        expression: Box::new(
                            DatexExpressionData::Identifier(
                                "myRef".to_string()
                            )
                            .with_default_span()
                        ),
                    })
                    .with_default_span()
                ),
            })
        );
    }

    #[test]
    fn parse_dereference_addition() {
        let expr = parse("*myRef + *myRef2");
        assert_eq!(
            expr.data,
            DatexExpressionData::BinaryOperation(BinaryOperation {
                left: Box::new(
                    DatexExpressionData::Deref(Deref {
                        expression: Box::new(
                            DatexExpressionData::Identifier(
                                "myRef".to_string()
                            )
                            .with_default_span()
                        ),
                    })
                    .with_default_span()
                ),
                operator: BinaryOperator::Arithmetic(ArithmeticOperator::Add),
                right: Box::new(
                    DatexExpressionData::Deref(Deref {
                        expression: Box::new(
                            DatexExpressionData::Identifier(
                                "myRef2".to_string()
                            )
                            .with_default_span()
                        ),
                    })
                    .with_default_span()
                ),
                ty: None,
            })
        );
    }

    #[test]
    fn parse_variable_assignment() {
        let expr = parse("x = true");
        assert_eq!(
            expr.data,
            DatexExpressionData::VariableAssignment(VariableAssignment {
                id: None,
                name: "x".to_string(),
                operator: AssignmentOperator::Assign,
                expression: Box::new(
                    DatexExpressionData::Boolean(true).with_default_span()
                ),
            })
        );
    }

    #[test]
    fn parse_invalid_variable_assignment() {
        let result = try_parse_and_return_on_first_error("42 = true");
        assert!(result.is_err());
        assert_eq!(
            result.err().unwrap().error,
            ParserError::InvalidAssignmentTarget
        );
    }

    #[test]
    fn parse_invalid_assignment_with_lhs_expression() {
        let result = try_parse_and_return_on_first_error("x + y = true");
        assert!(result.is_err());
        assert_eq!(
            result.err().unwrap().error,
            ParserError::InvalidAssignmentTarget
        );
    }

    #[test]
    fn parse_property_assignment() {
        let expr = parse("obj.prop = true");
        assert_eq!(
            expr.data,
            DatexExpressionData::PropertyAssignment(PropertyAssignment {
                operator: AssignmentOperator::Assign,
                base: Box::new(
                    DatexExpressionData::Identifier("obj".to_string())
                        .with_default_span()
                ),
                property: Box::new(
                    DatexExpressionData::Text("prop".to_string())
                        .with_default_span()
                ),
                assigned_expression: Box::new(
                    DatexExpressionData::Boolean(true).with_default_span()
                ),
            })
        );
    }

    #[test]
    fn parse_variable_add_assignment() {
        let expr = parse("x += 42");
        assert_eq!(
            expr.data,
            DatexExpressionData::VariableAssignment(VariableAssignment {
                id: None,
                name: "x".to_string(),
                operator: AssignmentOperator::AddAssign,
                expression: Box::new(
                    DatexExpressionData::Integer(42.into()).with_default_span()
                ),
            })
        );
    }

    #[test]
    fn parse_slot_assignment() {
        let expr = parse("#slot1 = 100");
        assert_eq!(
            expr.data,
            DatexExpressionData::SlotAssignment(SlotAssignment {
                slot: Slot::Named("slot1".to_string()),
                expression: Box::new(
                    DatexExpressionData::Integer(100.into())
                        .with_default_span()
                ),
            })
        );
    }

    #[test]
    fn parse_deref_assignment() {
        let expr = parse("*myRef = 200");
        assert_eq!(
            expr.data,
            DatexExpressionData::DerefAssignment(DerefAssignment {
                operator: AssignmentOperator::Assign,
                deref_expression: Box::new(
                    DatexExpressionData::Identifier("myRef".to_string())
                        .with_default_span()
                ),
                assigned_expression: Box::new(
                    DatexExpressionData::Integer(200.into())
                        .with_default_span()
                ),
            })
        );
    }

    #[test]
    fn parse_nested_deref_assignment() {
        let expr = parse("**myRef = 300");
        assert_eq!(
            expr.data,
            DatexExpressionData::DerefAssignment(DerefAssignment {
                operator: AssignmentOperator::Assign,
                deref_expression: Box::new(
                    DatexExpressionData::Deref(Deref {
                        expression: Box::new(
                            DatexExpressionData::Identifier(
                                "myRef".to_string()
                            )
                            .with_default_span()
                        ),
                    })
                    .with_default_span()
                ),
                assigned_expression: Box::new(
                    DatexExpressionData::Integer(300.into())
                        .with_default_span()
                ),
            })
        );
    }

    #[test]
    fn parse_structural_equality_comparison() {
        let expr = parse("a == b");
        assert_eq!(
            expr.data,
            DatexExpressionData::ComparisonOperation(ComparisonOperation {
                left: Box::new(
                    DatexExpressionData::Identifier("a".to_string())
                        .with_default_span()
                ),
                operator: ComparisonOperator::StructuralEqual,
                right: Box::new(
                    DatexExpressionData::Identifier("b".to_string())
                        .with_default_span()
                ),
            })
        );
    }

    #[test]
    fn parse_is_comparison() {
        let expr = parse("a is b");
        assert_eq!(
            expr.data,
            DatexExpressionData::ComparisonOperation(ComparisonOperation {
                left: Box::new(
                    DatexExpressionData::Identifier("a".to_string())
                        .with_default_span()
                ),
                operator: ComparisonOperator::Is,
                right: Box::new(
                    DatexExpressionData::Identifier("b".to_string())
                        .with_default_span()
                ),
            })
        );
    }

    #[test]
    fn parse_equality_comparison() {
        let expr = parse("a === b");
        assert_eq!(
            expr.data,
            DatexExpressionData::ComparisonOperation(ComparisonOperation {
                left: Box::new(
                    DatexExpressionData::Identifier("a".to_string())
                        .with_default_span()
                ),
                operator: ComparisonOperator::Equal,
                right: Box::new(
                    DatexExpressionData::Identifier("b".to_string())
                        .with_default_span()
                ),
            })
        );
    }

    #[test]
    fn parse_structural_inequality_comparison() {
        let expr = parse("a != b");
        assert_eq!(
            expr.data,
            DatexExpressionData::ComparisonOperation(ComparisonOperation {
                left: Box::new(
                    DatexExpressionData::Identifier("a".to_string())
                        .with_default_span()
                ),
                operator: ComparisonOperator::NotStructuralEqual,
                right: Box::new(
                    DatexExpressionData::Identifier("b".to_string())
                        .with_default_span()
                ),
            })
        );
    }

    #[test]
    fn parse_inequality_comparison() {
        let expr = parse("a !== b");
        assert_eq!(
            expr.data,
            DatexExpressionData::ComparisonOperation(ComparisonOperation {
                left: Box::new(
                    DatexExpressionData::Identifier("a".to_string())
                        .with_default_span()
                ),
                operator: ComparisonOperator::NotEqual,
                right: Box::new(
                    DatexExpressionData::Identifier("b".to_string())
                        .with_default_span()
                ),
            })
        );
    }

    #[test]
    fn parse_less_than_comparison() {
        let expr = parse("a < b");
        assert_eq!(
            expr.data,
            DatexExpressionData::ComparisonOperation(ComparisonOperation {
                left: Box::new(
                    DatexExpressionData::Identifier("a".to_string())
                        .with_default_span()
                ),
                operator: ComparisonOperator::LessThan,
                right: Box::new(
                    DatexExpressionData::Identifier("b".to_string())
                        .with_default_span()
                ),
            })
        );
    }

    #[test]
    fn parse_generic_instantiation() {
        let expr = parse("MyType<Arg1>");
        assert_eq!(
            expr.data,
            DatexExpressionData::GenericInstantiation(GenericInstantiation {
                base: Box::new(
                    DatexExpressionData::Identifier("MyType".to_string())
                        .with_default_span()
                ),
                generic_arguments: vec![
                    TypeExpressionData::Identifier("Arg1".to_string())
                        .with_default_span(),
                ],
            })
        );
    }

    #[test]
    fn parse_generic_instantiation_multiple_arguments() {
        let expr = parse("MyType<Arg1, Arg2>");
        assert_eq!(
            expr.data,
            DatexExpressionData::GenericInstantiation(GenericInstantiation {
                base: Box::new(
                    DatexExpressionData::Identifier("MyType".to_string())
                        .with_default_span()
                ),
                generic_arguments: vec![
                    TypeExpressionData::Identifier("Arg1".to_string())
                        .with_default_span(),
                    TypeExpressionData::Identifier("Arg2".to_string())
                        .with_default_span(),
                ],
            })
        );
    }

    #[test]
    fn parse_generic_instantiation_with_apply() {
        let expr = parse("MyType<Arg1>(42)");
        assert_eq!(
            expr.data,
            DatexExpressionData::Apply(Apply {
                base: Box::new(
                    DatexExpressionData::GenericInstantiation(
                        GenericInstantiation {
                            base: Box::new(
                                DatexExpressionData::Identifier(
                                    "MyType".to_string()
                                )
                                .with_default_span()
                            ),
                            generic_arguments: vec![
                                TypeExpressionData::Identifier(
                                    "Arg1".to_string()
                                )
                                .with_default_span(),
                            ],
                        }
                    )
                    .with_default_span()
                ),
                arguments: vec![
                    DatexExpressionData::Integer(42.into()).with_default_span(),
                ],
            })
        );
    }

    #[test]
    fn parse_greater_than_comparison() {
        let expr = parse("a > b");
        assert_eq!(
            expr.data,
            DatexExpressionData::ComparisonOperation(ComparisonOperation {
                left: Box::new(
                    DatexExpressionData::Identifier("a".to_string())
                        .with_default_span()
                ),
                operator: ComparisonOperator::GreaterThan,
                right: Box::new(
                    DatexExpressionData::Identifier("b".to_string())
                        .with_default_span()
                ),
            })
        );
    }

    #[test]
    fn parse_less_equal_comparison() {
        let expr = parse("a <= b");
        assert_eq!(
            expr.data,
            DatexExpressionData::ComparisonOperation(ComparisonOperation {
                left: Box::new(
                    DatexExpressionData::Identifier("a".to_string())
                        .with_default_span()
                ),
                operator: ComparisonOperator::LessThanOrEqual,
                right: Box::new(
                    DatexExpressionData::Identifier("b".to_string())
                        .with_default_span()
                ),
            })
        );
    }

    #[test]
    fn parse_greater_equal_comparison() {
        let expr = parse("a >= b");
        assert_eq!(
            expr.data,
            DatexExpressionData::ComparisonOperation(ComparisonOperation {
                left: Box::new(
                    DatexExpressionData::Identifier("a".to_string())
                        .with_default_span()
                ),
                operator: ComparisonOperator::GreaterThanOrEqual,
                right: Box::new(
                    DatexExpressionData::Identifier("b".to_string())
                        .with_default_span()
                ),
            })
        );
    }

    #[test]
    fn parse_comparison_precedence() {
        let expr = parse("a == b + c");
        assert_eq!(
            expr.data,
            DatexExpressionData::ComparisonOperation(ComparisonOperation {
                left: Box::new(
                    DatexExpressionData::Identifier("a".to_string())
                        .with_default_span()
                ),
                operator: ComparisonOperator::StructuralEqual,
                right: Box::new(
                    DatexExpressionData::BinaryOperation(BinaryOperation {
                        left: Box::new(
                            DatexExpressionData::Identifier("b".to_string())
                                .with_default_span()
                        ),
                        operator: BinaryOperator::Arithmetic(
                            ArithmeticOperator::Add
                        ),
                        right: Box::new(
                            DatexExpressionData::Identifier("c".to_string())
                                .with_default_span()
                        ),
                        ty: None,
                    })
                    .with_default_span()
                ),
            })
        );
    }

    #[test]
    fn parse_negation() {
        let expr = parse("!true");
        assert_eq!(
            expr.data,
            DatexExpressionData::UnaryOperation(UnaryOperation {
                operator: UnaryOperator::Logical(LogicalUnaryOperator::Not),
                expression: Box::new(
                    DatexExpressionData::Boolean(true).with_default_span()
                ),
            })
        );
    }

    #[test]
    fn parse_unary_plus_integer() {
        let expr = parse("+42");
        assert_eq!(expr.data, DatexExpressionData::Integer(42.into()));
    }

    #[test]
    fn parse_unary_plus_variable() {
        let expr = parse("+x");
        assert_eq!(
            expr.data,
            DatexExpressionData::UnaryOperation(UnaryOperation {
                operator: UnaryOperator::Arithmetic(
                    ArithmeticUnaryOperator::Plus
                ),
                expression: Box::new(
                    DatexExpressionData::Identifier("x".to_string())
                        .with_default_span()
                ),
            })
        );
    }

    #[test]
    fn parse_unary_minus_integer() {
        let expr = parse("-42");
        assert_eq!(expr.data, DatexExpressionData::Integer((-42).into()));
    }

    #[test]
    fn parse_unary_minus_variable() {
        let expr = parse("-x");
        assert_eq!(
            expr.data,
            DatexExpressionData::UnaryOperation(UnaryOperation {
                operator: UnaryOperator::Arithmetic(
                    ArithmeticUnaryOperator::Minus
                ),
                expression: Box::new(
                    DatexExpressionData::Identifier("x".to_string())
                        .with_default_span()
                ),
            })
        );
    }

    #[test]
    fn parse_power_expression() {
        let expr = parse("2 ^ 3");
        assert_eq!(
            expr.data,
            DatexExpressionData::BinaryOperation(BinaryOperation {
                left: Box::new(
                    DatexExpressionData::Integer(2.into()).with_default_span()
                ),
                operator: BinaryOperator::Arithmetic(ArithmeticOperator::Power),
                right: Box::new(
                    DatexExpressionData::Integer(3.into()).with_default_span()
                ),
                ty: None,
            })
        );
    }

    #[test]
    fn parse_power_expression_right_associative() {
        let expr = parse("2 ^ 3 ^ 4");
        assert_eq!(
            expr.data,
            DatexExpressionData::BinaryOperation(BinaryOperation {
                left: Box::new(
                    DatexExpressionData::Integer(2.into()).with_default_span()
                ),
                operator: BinaryOperator::Arithmetic(ArithmeticOperator::Power),
                right: Box::new(
                    DatexExpressionData::BinaryOperation(BinaryOperation {
                        left: Box::new(
                            DatexExpressionData::Integer(3.into())
                                .with_default_span()
                        ),
                        operator: BinaryOperator::Arithmetic(
                            ArithmeticOperator::Power
                        ),
                        right: Box::new(
                            DatexExpressionData::Integer(4.into())
                                .with_default_span()
                        ),
                        ty: None,
                    })
                    .with_default_span()
                ),
                ty: None,
            })
        );
    }

    #[test]
    fn parse_power_expression_with_parentheses() {
        let expr = parse("(2 ^ 3) ^ 4");
        assert_eq!(
            expr.data,
            DatexExpressionData::BinaryOperation(BinaryOperation {
                left: Box::new(
                    DatexExpressionData::BinaryOperation(BinaryOperation {
                        left: Box::new(
                            DatexExpressionData::Integer(2.into())
                                .with_default_span()
                        ),
                        operator: BinaryOperator::Arithmetic(
                            ArithmeticOperator::Power
                        ),
                        right: Box::new(
                            DatexExpressionData::Integer(3.into())
                                .with_default_span()
                        ),
                        ty: None,
                    })
                    .with_default_span()
                ),
                operator: BinaryOperator::Arithmetic(ArithmeticOperator::Power),
                right: Box::new(
                    DatexExpressionData::Integer(4.into()).with_default_span()
                ),
                ty: None,
            })
        );
    }
}
