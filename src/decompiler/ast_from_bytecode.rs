use crate::ast::spanned::Spanned;
use crate::ast::structs::expression::{
    BinaryOperation, DatexExpression, List, Map, UnaryOperation,
};
use crate::ast::structs::expression::{DatexExpressionData, Statements};
use crate::ast::structs::r#type::{TypeExpression, TypeExpressionData};
use crate::global::operators::{BinaryOperator, UnaryOperator};
use crate::global::protocol_structures::instructions::{
    Instruction, RegularInstruction, TypeInstruction,
};
use crate::parser::body::{DXBParserError, iterate_instructions};
use crate::parser::instruction_collector::{
    CollectedResults, CollectionResultsPopper, FullOrPartialResult,
    InstructionCollector,
};
use crate::runtime::execution::execution_loop::interrupts::{
    ExecutionInterrupt, ExternalExecutionInterrupt,
};
use crate::stdlib::rc::Rc;
use crate::values::core_values::decimal::Decimal;
use crate::values::core_values::decimal::typed_decimal::TypedDecimal;
use crate::values::core_values::integer::typed_integer::TypedInteger;
use crate::values::pointer::PointerAddress;
use core::cell::RefCell;
use datex_core::ast::structs::apply_operation::ApplyOperation;
use datex_core::ast::structs::expression::{ApplyChain, UnboundedStatement};
use datex_core::parser::instruction_collector::StatementResultCollectionStrategy;

#[derive(Debug)]
enum CollectedAstResult {
    Expression(DatexExpression),
    TypeExpression(TypeExpression),
    KeyValuePair((DatexExpression, DatexExpression)),
}
impl From<DatexExpression> for CollectedAstResult {
    fn from(value: DatexExpression) -> Self {
        CollectedAstResult::Expression(value)
    }
}

impl From<TypeExpression> for CollectedAstResult {
    fn from(value: TypeExpression) -> Self {
        CollectedAstResult::TypeExpression(value)
    }
}

impl
    CollectionResultsPopper<
        CollectedAstResult,
        DatexExpression,
        DatexExpression,
        TypeExpression,
    > for CollectedResults<CollectedAstResult>
{
    /// Pops a DatexExpression from the collected results.
    fn try_extract_value_result(
        result: CollectedAstResult,
    ) -> Option<DatexExpression> {
        match result {
            CollectedAstResult::Expression(expr) => Some(expr),
            _ => None,
        }
    }

    /// Pops a TypeExpression from the collected results.
    fn try_extract_type_result(
        result: CollectedAstResult,
    ) -> Option<TypeExpression> {
        match result {
            CollectedAstResult::TypeExpression(expr) => Some(expr),
            _ => None,
        }
    }

    /// Pops a key-value pair from the collected results.
    fn try_extract_key_value_pair_result(
        result: CollectedAstResult,
    ) -> Option<(DatexExpression, DatexExpression)> {
        match result {
            CollectedAstResult::KeyValuePair((key, value)) => {
                Some((key, value))
            }
            _ => None,
        }
    }
}

pub fn ast_from_bytecode(
    dxb: &[u8],
) -> Result<DatexExpression, DXBParserError> {
    let mut collector = InstructionCollector::<CollectedAstResult>::default();

    for instruction in iterate_instructions(Rc::new(RefCell::new(dxb.to_vec())))
    {
        let instruction = instruction?;

        let result = match instruction {
            // handle regular instructions
            Instruction::RegularInstruction(regular_instruction) => {
                let regular_instruction = collector
                    .default_regular_instruction_collection(
                        regular_instruction,
                        StatementResultCollectionStrategy::Full,
                    );

                let expr: Option<DatexExpression> =
                    if let Some(regular_instruction) = regular_instruction {
                        Some(
                            match regular_instruction {
                                // Handle different regular instructions here
                                RegularInstruction::Int8(integer_data) => {
                                    DatexExpressionData::TypedInteger(
                                        TypedInteger::from(integer_data.0),
                                    )
                                }
                                RegularInstruction::Int16(integer_data) => {
                                    DatexExpressionData::TypedInteger(
                                        TypedInteger::from(integer_data.0),
                                    )
                                }
                                RegularInstruction::Int32(integer_data) => {
                                    DatexExpressionData::TypedInteger(
                                        TypedInteger::from(integer_data.0),
                                    )
                                }
                                RegularInstruction::Int64(integer_data) => {
                                    DatexExpressionData::TypedInteger(
                                        TypedInteger::from(integer_data.0),
                                    )
                                }
                                RegularInstruction::Int128(integer_data) => {
                                    DatexExpressionData::TypedInteger(
                                        TypedInteger::from(integer_data.0),
                                    )
                                }
                                RegularInstruction::UInt8(integer_data) => {
                                    DatexExpressionData::TypedInteger(
                                        TypedInteger::from(integer_data.0),
                                    )
                                }
                                RegularInstruction::UInt16(integer_data) => {
                                    DatexExpressionData::TypedInteger(
                                        TypedInteger::from(integer_data.0),
                                    )
                                }
                                RegularInstruction::UInt32(integer_data) => {
                                    DatexExpressionData::TypedInteger(
                                        TypedInteger::from(integer_data.0),
                                    )
                                }
                                RegularInstruction::UInt64(integer_data) => {
                                    DatexExpressionData::TypedInteger(
                                        TypedInteger::from(integer_data.0),
                                    )
                                }
                                RegularInstruction::UInt128(integer_data) => {
                                    DatexExpressionData::TypedInteger(
                                        TypedInteger::from(integer_data.0),
                                    )
                                }
                                RegularInstruction::BigInteger(
                                    integer_data,
                                ) => DatexExpressionData::TypedInteger(
                                    TypedInteger::Big(integer_data.0),
                                ),
                                RegularInstruction::Integer(integer_data) => {
                                    DatexExpressionData::Integer(integer_data.0)
                                }
                                RegularInstruction::Endpoint(endpoint) => {
                                    DatexExpressionData::Endpoint(endpoint)
                                }
                                RegularInstruction::DecimalF32(f32_data) => {
                                    DatexExpressionData::TypedDecimal(
                                        TypedDecimal::from(f32_data.0),
                                    )
                                }
                                RegularInstruction::DecimalF64(f64_data) => {
                                    DatexExpressionData::TypedDecimal(
                                        TypedDecimal::from(f64_data.0),
                                    )
                                }
                                RegularInstruction::DecimalAsInt16(
                                    decimal_i16_data,
                                ) => DatexExpressionData::Decimal(
                                    Decimal::from(decimal_i16_data.0 as f64),
                                ),
                                RegularInstruction::DecimalAsInt32(
                                    decimal_i32_data,
                                ) => DatexExpressionData::Decimal(
                                    Decimal::from(decimal_i32_data.0 as f64),
                                ),
                                RegularInstruction::BigDecimal(
                                    decimal_data,
                                ) => DatexExpressionData::TypedDecimal(
                                    TypedDecimal::Decimal(decimal_data.0),
                                ),
                                RegularInstruction::Decimal(decimal_data) => {
                                    DatexExpressionData::Decimal(decimal_data.0)
                                }
                                RegularInstruction::ShortText(
                                    short_text_data,
                                ) => {
                                    DatexExpressionData::Text(short_text_data.0)
                                }
                                RegularInstruction::Text(text_data) => {
                                    DatexExpressionData::Text(text_data.0)
                                }
                                RegularInstruction::True => {
                                    DatexExpressionData::Boolean(true)
                                }
                                RegularInstruction::False => {
                                    DatexExpressionData::Boolean(false)
                                }
                                RegularInstruction::Null => {
                                    DatexExpressionData::Null
                                }

                                // NOTE: make sure that each possible match case is either implemented in the default collection or here
                                // If an instruction is implemented in the default collection, it should be marked as unreachable!() here
                                RegularInstruction::Statements(_)
                                | RegularInstruction::ShortStatements(_)
                                | RegularInstruction::UnboundedStatements
                                | RegularInstruction::UnboundedStatementsEnd(
                                    _,
                                )
                                | RegularInstruction::List(_)
                                | RegularInstruction::ShortList(_)
                                | RegularInstruction::Map(_)
                                | RegularInstruction::ShortMap(_)
                                | RegularInstruction::KeyValueDynamic
                                | RegularInstruction::KeyValueShortText(_)
                                | RegularInstruction::Add
                                | RegularInstruction::Subtract
                                | RegularInstruction::Multiply
                                | RegularInstruction::Divide
                                | RegularInstruction::UnaryMinus
                                | RegularInstruction::UnaryPlus
                                | RegularInstruction::BitwiseNot
                                | RegularInstruction::Apply(_)
                                | RegularInstruction::Is
                                | RegularInstruction::Matches
                                | RegularInstruction::StructuralEqual
                                | RegularInstruction::Equal
                                | RegularInstruction::NotStructuralEqual
                                | RegularInstruction::NotEqual
                                | RegularInstruction::AddAssign(_)
                                | RegularInstruction::SubtractAssign(_)
                                | RegularInstruction::MultiplyAssign(_)
                                | RegularInstruction::DivideAssign(_)
                                | RegularInstruction::CreateRef
                                | RegularInstruction::CreateRefMut
                                | RegularInstruction::GetRef(_)
                                | RegularInstruction::GetLocalRef(_)
                                | RegularInstruction::GetInternalRef(_)
                                | RegularInstruction::GetOrCreateRef(_)
                                | RegularInstruction::GetOrCreateRefMut(_)
                                | RegularInstruction::AllocateSlot(_)
                                | RegularInstruction::GetSlot(_)
                                | RegularInstruction::DropSlot(_)
                                | RegularInstruction::SetSlot(_)
                                | RegularInstruction::SetReferenceValue(_)
                                | RegularInstruction::Deref
                                | RegularInstruction::TypedValue
                                | RegularInstruction::RemoteExecution(_)
                                | RegularInstruction::TypeExpression => {
                                    unreachable!()
                                }
                            }
                            .with_default_span(),
                        )
                    } else {
                        None
                    };

                expr.map(|expr| CollectedAstResult::from(expr))
            }
            Instruction::TypeInstruction(type_instruction) => {
                let type_instruction = collector
                    .default_type_instruction_collection(type_instruction);

                let type_expression: Option<TypeExpression> =
                    if let Some(type_instruction) = type_instruction {
                        Some(
                            match type_instruction {
                                TypeInstruction::LiteralInteger(
                                    integer_data,
                                ) => {
                                    TypeExpressionData::Integer(integer_data.0)
                                }
                                TypeInstruction::LiteralText(text_data) => {
                                    TypeExpressionData::Text(text_data.0)
                                }
                                TypeInstruction::TypeReference(reference) => {
                                    TypeExpressionData::GetReference(
                                        PointerAddress::from(reference.address),
                                    )
                                }
                                // NOTE: make sure that each possible match case is either implemented in the default collection or here
                                // If an instruction is implemented in the default collection, it should be marked as unreachable!() here
                                TypeInstruction::List(_)
                                | TypeInstruction::ImplType(_) => {
                                    unreachable!()
                                }
                            }
                            .with_default_span(),
                        )
                    } else {
                        None
                    };

                type_expression.map(|ty_expr| CollectedAstResult::from(ty_expr))
            }
        };

        if let Some(result) = result {
            collector.push_result(result);
        }

        // handle collecting nested expressions
        while let Some(result) = collector.try_pop_collected() {
            match result {
                FullOrPartialResult::Full(
                    instruction,
                    mut collected_results,
                ) => {
                    let expr: CollectedAstResult = match instruction {
                        Instruction::RegularInstruction(
                            regular_instruction,
                        ) => match regular_instruction {
                            RegularInstruction::List(_)
                            | RegularInstruction::ShortList(_) => {
                                let elements =
                                    collected_results.collect_value_results();
                                DatexExpressionData::List(List::new(elements))
                                    .with_default_span()
                                    .into()
                            }
                            RegularInstruction::Map(_)
                            | RegularInstruction::ShortMap(_) => {
                                let entries = collected_results
                                    .collect_key_value_pair_results();
                                DatexExpressionData::Map(Map::new(entries))
                                    .with_default_span()
                                    .into()
                            }
                            RegularInstruction::Statements(statements_data)
                            | RegularInstruction::ShortStatements(
                                statements_data,
                            ) => {
                                let statements =
                                    collected_results.collect_value_results();
                                DatexExpressionData::Statements(Statements {
                                    statements,
                                    is_terminated: statements_data.terminated,
                                    unbounded: None,
                                })
                                .with_default_span()
                                .into()
                            }

                            RegularInstruction::KeyValueDynamic => {
                                let value =
                                    collected_results.pop_value_result();
                                let key = collected_results.pop_value_result();
                                CollectedAstResult::KeyValuePair((key, value))
                            }

                            RegularInstruction::KeyValueShortText(
                                short_text_data,
                            ) => {
                                let value =
                                    collected_results.pop_value_result();
                                let key = DatexExpressionData::Text(
                                    short_text_data.0,
                                )
                                .with_default_span();
                                CollectedAstResult::KeyValuePair((key, value))
                            }

                            RegularInstruction::Add
                            | RegularInstruction::Subtract
                            | RegularInstruction::Multiply
                            | RegularInstruction::Divide
                            | RegularInstruction::Matches
                            | RegularInstruction::StructuralEqual
                            | RegularInstruction::Equal
                            | RegularInstruction::NotStructuralEqual
                            | RegularInstruction::NotEqual => {
                                let right =
                                    collected_results.pop_value_result();
                                let left = collected_results.pop_value_result();
                                DatexExpressionData::BinaryOperation(
                                    BinaryOperation {
                                        operator: BinaryOperator::from(
                                            &regular_instruction,
                                        ),
                                        left: Box::new(left),
                                        right: Box::new(right),
                                        ty: None,
                                    },
                                )
                                .with_default_span()
                                .into()
                            }

                            RegularInstruction::UnaryMinus
                            | RegularInstruction::UnaryPlus
                            | RegularInstruction::BitwiseNot
                            | RegularInstruction::CreateRef
                            | RegularInstruction::CreateRefMut
                            | RegularInstruction::Deref => {
                                let expr = collected_results.pop_value_result();
                                DatexExpressionData::UnaryOperation(
                                    UnaryOperation {
                                        operator: UnaryOperator::from(
                                            &regular_instruction,
                                        ),
                                        expression: Box::new(expr),
                                    },
                                )
                                .with_default_span()
                                .into()
                            }

                            RegularInstruction::TypedValue => {
                                let expr = collected_results.pop_value_result();
                                let expr_type =
                                    collected_results.pop_type_result();
                                DatexExpressionData::ApplyChain(ApplyChain {
                                    base: Box::new(DatexExpressionData::TypeExpression(expr_type).with_default_span()),
                                    operations: vec![ApplyOperation::FunctionCallSingleArgument(expr)],
                                }).with_default_span().into()
                            }

                            RegularInstruction::UnboundedStatementsEnd(
                                terminated,
                            ) => {
                                let result = collector.try_pop_unbounded().ok_or(DXBParserError::NotInUnboundedRegularScopeError)?;
                                if let FullOrPartialResult::Full(
                                    _,
                                    mut results,
                                ) = result
                                {
                                    DatexExpressionData::Statements(
                                        Statements {
                                            statements: results
                                                .collect_value_results(),
                                            is_terminated: terminated,
                                            unbounded: Some(
                                                UnboundedStatement {
                                                    is_first: true,
                                                    is_last: true,
                                                },
                                            ),
                                        },
                                    )
                                    .with_default_span()
                                    .into()
                                } else {
                                    unreachable!()
                                }
                            }

                            e => {
                                todo!(
                                    "Unhandled collected regular instruction: {:?}",
                                    e
                                );
                            }
                        },

                        Instruction::TypeInstruction(data) => {
                            todo!()
                        }
                    };
                    collector.push_result(expr);
                }
                _ => unreachable!(),
            }
        }
    }

    if let Some(result) = collector.take_root_result() {
        match result {
            CollectedAstResult::Expression(expr) => Ok(expr),
            _ => unreachable!("Expected root result"),
        }
    } else {
        panic!("Execution finished without root result");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::global::operators::binary::ArithmeticOperator;
    use crate::global::type_instruction_codes::TypeInstructionCode;
    use crate::{
        ast::spanned::Spanned, global::instruction_codes::InstructionCode,
    };

    #[test]
    fn ast_from_bytecode_simple_integer() {
        let bytecode: Vec<u8> = vec![InstructionCode::UINT_8 as u8, 0x2A];
        let ast = ast_from_bytecode(&bytecode).unwrap();
        assert_eq!(
            ast,
            DatexExpressionData::TypedInteger(TypedInteger::from(42u8))
                .with_default_span()
        );
    }

    #[test]
    fn ast_from_bytecode_null() {
        let bytecode: Vec<u8> = vec![InstructionCode::NULL as u8];
        let ast = ast_from_bytecode(&bytecode).unwrap();
        assert_eq!(ast, DatexExpressionData::Null.with_default_span());
    }

    #[test]
    fn ast_from_bytecode_simple_boolean() {
        let bytecode: Vec<u8> = vec![InstructionCode::TRUE as u8];
        let ast = ast_from_bytecode(&bytecode).unwrap();
        assert_eq!(ast, DatexExpressionData::Boolean(true).with_default_span());
    }

    #[test]
    fn ast_from_bytecode_simple_text() {
        let bytecode: Vec<u8> = vec![
            InstructionCode::SHORT_TEXT as u8,
            0x05, // length 5
            b'H',
            b'e',
            b'l',
            b'l',
            b'o',
        ];
        let ast = ast_from_bytecode(&bytecode).unwrap();
        assert_eq!(
            ast,
            DatexExpressionData::Text("Hello".to_string()).with_default_span()
        );
    }

    #[test]
    fn ast_from_bytecode_simple_list() {
        let bytecode: Vec<u8> = vec![
            InstructionCode::SHORT_LIST as u8,
            0x02, // 2 elements
            InstructionCode::UINT_8 as u8,
            0x2A, // 42
            InstructionCode::UINT_8 as u8,
            0x15, // 21
        ];
        let ast = ast_from_bytecode(&bytecode).unwrap();
        assert_eq!(
            ast,
            DatexExpressionData::List(List::new(vec![
                DatexExpressionData::TypedInteger(TypedInteger::from(42u8))
                    .with_default_span(),
                DatexExpressionData::TypedInteger(TypedInteger::from(21u8))
                    .with_default_span(),
            ]))
            .with_default_span()
        );
    }

    #[test]
    fn ast_from_bytecode_nested_list() {
        let bytecode: Vec<u8> = vec![
            InstructionCode::SHORT_LIST as u8,
            0x02, // 2 elements
            InstructionCode::SHORT_LIST as u8,
            0x02, // 2 elements
            InstructionCode::UINT_8 as u8,
            0x01, // 1
            InstructionCode::UINT_8 as u8,
            0x02, // 2
            InstructionCode::UINT_8 as u8,
            0x03, // 3
        ];
        let ast = ast_from_bytecode(&bytecode).unwrap();
        assert_eq!(
            ast,
            DatexExpressionData::List(List::new(vec![
                DatexExpressionData::List(List::new(vec![
                    DatexExpressionData::TypedInteger(TypedInteger::from(1u8))
                        .with_default_span(),
                    DatexExpressionData::TypedInteger(TypedInteger::from(2u8))
                        .with_default_span(),
                ]))
                .with_default_span(),
                DatexExpressionData::TypedInteger(TypedInteger::from(3u8))
                    .with_default_span(),
            ]))
            .with_default_span()
        );
    }

    #[test]
    fn ast_from_bytecode_statements() {
        let bytecode: Vec<u8> = vec![
            InstructionCode::SHORT_STATEMENTS as u8,
            0x02, // 2 statements
            0x01, // terminated
            InstructionCode::UINT_8 as u8,
            42,
            InstructionCode::UINT_8 as u8,
            21,
        ];
        let ast = ast_from_bytecode(&bytecode).unwrap();
        assert_eq!(
            ast,
            DatexExpressionData::Statements(Statements {
                statements: vec![
                    DatexExpressionData::TypedInteger(TypedInteger::from(42u8))
                        .with_default_span(),
                    DatexExpressionData::TypedInteger(TypedInteger::from(21u8))
                        .with_default_span(),
                ],
                is_terminated: true,
                unbounded: None,
            })
            .with_default_span()
        );
    }

    #[test]
    fn ast_from_nested_expressions() {
        let bytecode: Vec<u8> = vec![
            InstructionCode::SHORT_LIST as u8,
            0x03, // 3 elements
            InstructionCode::UINT_8 as u8,
            0x01, // 1
            InstructionCode::UINT_8 as u8,
            0x02, // 2
            InstructionCode::ADD as u8,
            InstructionCode::UINT_8 as u8,
            0x03, // 3
            InstructionCode::UINT_8 as u8,
            0x04, // 4
        ];
        let ast = ast_from_bytecode(&bytecode).unwrap();
        assert_eq!(
            ast,
            DatexExpressionData::List(List::new(vec![
                DatexExpressionData::TypedInteger(TypedInteger::from(1u8))
                    .with_default_span(),
                DatexExpressionData::TypedInteger(TypedInteger::from(2u8))
                    .with_default_span(),
                DatexExpressionData::BinaryOperation(BinaryOperation {
                    operator: BinaryOperator::Arithmetic(
                        ArithmeticOperator::Add
                    ),
                    left: Box::new(
                        DatexExpressionData::TypedInteger(TypedInteger::from(
                            3u8
                        ))
                        .with_default_span()
                    ),
                    right: Box::new(
                        DatexExpressionData::TypedInteger(TypedInteger::from(
                            4u8
                        ))
                        .with_default_span()
                    ),
                    ty: None
                })
                .with_default_span(),
            ]))
            .with_default_span()
        );
    }

    #[test]
    fn typed_value() {
        let bytecode: Vec<u8> = vec![
            InstructionCode::TYPED_VALUE as u8,
            TypeInstructionCode::TYPE_LITERAL_SHORT_TEXT as u8,
            2,
            b'O',
            b'K',
            InstructionCode::UINT_8 as u8,
            43,
        ];
        let ast = ast_from_bytecode(&bytecode).unwrap();
        assert_eq!(
            ast,
            DatexExpressionData::ApplyChain(ApplyChain {
                base: Box::new(
                    DatexExpressionData::TypeExpression(
                        TypeExpressionData::Text("OK".to_string())
                            .with_default_span()
                    )
                    .with_default_span()
                ),
                operations: vec![ApplyOperation::FunctionCallSingleArgument(
                    DatexExpressionData::TypedInteger(TypedInteger::from(43u8))
                        .with_default_span()
                )],
            })
            .with_default_span()
        );
    }

    #[test]
    fn unbounded_statements() {
        let bytecode: Vec<u8> = vec![
            InstructionCode::UNBOUNDED_STATEMENTS as u8,
            InstructionCode::UINT_8 as u8,
            10,
            InstructionCode::UINT_8 as u8,
            20,
            InstructionCode::UNBOUNDED_STATEMENTS_END as u8,
            1, // terminated
        ];
        let ast = ast_from_bytecode(&bytecode).unwrap();
        assert_eq!(
            ast,
            DatexExpressionData::Statements(Statements {
                statements: vec![
                    DatexExpressionData::TypedInteger(TypedInteger::from(10u8))
                        .with_default_span(),
                    DatexExpressionData::TypedInteger(TypedInteger::from(20u8))
                        .with_default_span(),
                ],
                is_terminated: true,
                unbounded: Some(UnboundedStatement {
                    is_first: true,
                    is_last: true
                }),
            })
            .with_default_span()
        );
    }
}
