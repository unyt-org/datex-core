use crate::ast::spanned::Spanned;
use crate::ast::structs::expression::{BinaryOperation, DatexExpression, List, Map, UnaryOperation};
use crate::ast::structs::expression::{DatexExpressionData, Statements};
use crate::ast::structs::r#type::{TypeExpression, TypeExpressionData};
use crate::global::protocol_structures::instructions::{
    Instruction, RegularInstruction, TypeInstruction,
};
use crate::parser::body::{DXBParserError, iterate_instructions};
use crate::stdlib::rc::Rc;
use crate::values::core_values::decimal::Decimal;
use crate::values::core_values::decimal::typed_decimal::TypedDecimal;
use crate::values::core_values::integer::typed_integer::TypedInteger;
use crate::values::pointer::PointerAddress;
use core::cell::RefCell;
use crate::global::operators::binary::ArithmeticOperator;
use crate::global::operators::{BinaryOperator, UnaryOperator};

enum CollectedResult {
    Expression(DatexExpression),
    TypeExpression(TypeExpression),
}
impl From<DatexExpression> for CollectedResult {
    fn from(value: DatexExpression) -> Self {
        CollectedResult::Expression(value)
    }
}

impl From<TypeExpression> for CollectedResult {
    fn from(value: TypeExpression) -> Self {
        CollectedResult::TypeExpression(value)
    }
}

#[derive(Default)]
struct CollectedResults {
    results: Vec<CollectedResult>,
}

impl CollectedResults {
    fn pop(&mut self) -> Option<CollectedResult> {
        self.results.pop()
    }

    /// Pops a DatexExpression from the collected results.
    /// Panics if the popped result is not a DatexExpression.
    /// The caller must ensure that the next result is a DatexExpression and not a TypeExpression
    /// and that there is at least one result to pop.
    fn pop_expression(&mut self) -> DatexExpression {
        match self.pop() {
            Some(CollectedResult::Expression(expr)) => expr,
            _ => unreachable!("Expected DatexExpression in collected results")
        }
    }

    /// Collects all DatexExpressions from the collected results.
    /// Panics if any of the popped results are not DatexExpressions.
    fn collect_expressions(&mut self) -> Vec<DatexExpression> {
        let count = self.len();
        let mut expressions = Vec::with_capacity(count);
        for _ in 0..count {
            expressions.push(self.pop_expression());
        }
        expressions.reverse();
        expressions
    }

    /// Collects all TypeExpressions from the collected results.
    /// Panics if any of the popped results are not TypeExpressions.
    fn collect_type_expressions(&mut self) -> Vec<TypeExpression> {
        let count = self.len();
        let mut type_expressions = Vec::with_capacity(count);
        for _ in 0..count {
            type_expressions.push(self.pop_type_expression());
        }
        type_expressions.reverse();
        type_expressions
    }

    /// Pops a TypeExpression from the collected results.
    /// Panics if the popped result is not a TypeExpression.
    /// The caller must ensure that the next result is a TypeExpression and not a DatexExpression
    /// and that there is at least one result to pop.
    fn pop_type_expression(&mut self) -> TypeExpression {
        match self.pop() {
            Some(CollectedResult::TypeExpression(expr)) => expr,
            _ => unreachable!("Expected TypeExpression in collected results")
        }
    }

    fn push(&mut self, result: CollectedResult) {
        self.results.push(result);
    }

    fn len(&self) -> usize {
        self.results.len()
    }

    fn is_empty(&self) -> bool {
        self.results.is_empty()
    }
}


#[derive(Default)]
struct Collector {
    collectors: Vec<(Instruction, u32)>,
    results: CollectedResults,
}

impl Collector {
    fn collect(&mut self, instruction: Instruction, count: u32) {
        self.collectors.push((instruction, count));
    }

    fn is_collecting(&self) -> bool {
        !self.collectors.is_empty()
    }

    fn push_result(&mut self, result: impl Into<CollectedResult>) {
        self.results.push(result.into());
    }

    fn try_pop_collected(
        &mut self,
    ) -> Option<(Instruction, CollectedResults)> {
        let collector = self.collectors.last()?;
        let expected_count = collector.1;

        if self.results.len() as u32 == expected_count {
            let collector = self.collectors.pop().unwrap(); // we already checked if the last element exists
            Some((collector.0, core::mem::take(&mut self.results)))
        } else if self.results.len() as u32 > expected_count {
            panic!(
                "Collected more results than expected for the last instruction"
            );
        } else {
            None
        }
    }

    fn pop(&mut self) -> Option<CollectedResult> {
        self.results.pop()
    }

    fn pop_datex_expression(&mut self) -> Option<DatexExpression> {
        match self.pop() {
            Some(CollectedResult::Expression(expr)) => Some(expr),
            _ => None,
        }
    }
}

pub fn ast_from_bytecode(
    dxb: &[u8],
) -> Result<DatexExpression, DXBParserError> {
    let mut collector = Collector::default();

    for instruction in iterate_instructions(Rc::new(RefCell::new(dxb.to_vec())))
    {
        let instruction = instruction?;
        match instruction {
            // handle regular instructions
            Instruction::RegularInstruction(regular_instruction) => {
                let expr: Option<DatexExpression> = match regular_instruction {
                    // Handle different regular instructions here
                    RegularInstruction::Int8(integer_data) => Some(
                        DatexExpressionData::TypedInteger(TypedInteger::from(
                            integer_data.0,
                        ))
                        .with_default_span(),
                    ),
                    RegularInstruction::Int16(integer_data) => Some(
                        DatexExpressionData::TypedInteger(TypedInteger::from(
                            integer_data.0,
                        ))
                        .with_default_span(),
                    ),
                    RegularInstruction::Int32(integer_data) => Some(
                        DatexExpressionData::TypedInteger(TypedInteger::from(
                            integer_data.0,
                        ))
                        .with_default_span(),
                    ),
                    RegularInstruction::Int64(integer_data) => Some(
                        DatexExpressionData::TypedInteger(TypedInteger::from(
                            integer_data.0,
                        ))
                        .with_default_span(),
                    ),
                    RegularInstruction::Int128(integer_data) => Some(
                        DatexExpressionData::TypedInteger(TypedInteger::from(
                            integer_data.0,
                        ))
                            .with_default_span(),
                    ),
                    RegularInstruction::UInt8(integer_data) => Some(
                        DatexExpressionData::TypedInteger(TypedInteger::from(
                            integer_data.0,
                        ))
                        .with_default_span(),
                    ),
                    RegularInstruction::UInt16(integer_data) => Some(
                        DatexExpressionData::TypedInteger(TypedInteger::from(
                            integer_data.0,
                        ))
                        .with_default_span(),
                    ),
                    RegularInstruction::UInt32(integer_data) => Some(
                        DatexExpressionData::TypedInteger(TypedInteger::from(
                            integer_data.0,
                        ))
                        .with_default_span(),
                    ),
                    RegularInstruction::UInt64(integer_data) => Some(
                        DatexExpressionData::TypedInteger(TypedInteger::from(
                            integer_data.0,
                        ))
                        .with_default_span(),
                    ),
                    RegularInstruction::UInt128(integer_data) => Some(
                        DatexExpressionData::TypedInteger(TypedInteger::from(
                            integer_data.0,
                        ))
                            .with_default_span(),
                    ),
                    RegularInstruction::BigInteger(integer_data) => Some(
                        DatexExpressionData::TypedInteger(TypedInteger::Big(integer_data.0))
                            .with_default_span(),
                    ),
                    RegularInstruction::Integer(integer_data) => Some(
                        DatexExpressionData::Integer(integer_data.0)
                            .with_default_span(),
                    ),
                    RegularInstruction::Endpoint(endpoint) => Some(
                        DatexExpressionData::Endpoint(endpoint)
                            .with_default_span(),
                    ),
                    RegularInstruction::DecimalF32(f32_data) => Some(
                        DatexExpressionData::TypedDecimal(TypedDecimal::from(
                            f32_data.0,
                        ))
                        .with_default_span(),
                    ),
                    RegularInstruction::DecimalF64(f64_data) => Some(
                        DatexExpressionData::TypedDecimal(TypedDecimal::from(
                            f64_data.0,
                        ))
                        .with_default_span(),
                    ),
                    RegularInstruction::DecimalAsInt16(decimal_i16_data) => {
                        Some(
                            DatexExpressionData::Decimal(Decimal::from(
                                decimal_i16_data.0 as f64,
                            ))
                            .with_default_span(),
                        )
                    }
                    RegularInstruction::DecimalAsInt32(decimal_i32_data) => {
                        Some(
                            DatexExpressionData::Decimal(Decimal::from(
                                decimal_i32_data.0 as f64,
                            ))
                            .with_default_span(),
                        )
                    }
                    RegularInstruction::BigDecimal(decimal_data) => Some(
                        DatexExpressionData::TypedDecimal(TypedDecimal::Decimal(decimal_data.0))
                            .with_default_span(),
                    ),
                    RegularInstruction::Decimal(decimal_data) => Some(
                        DatexExpressionData::Decimal(decimal_data.0)
                            .with_default_span(),
                    ),
                    RegularInstruction::RemoteExecution(_) => todo!(),
                    RegularInstruction::ShortText(short_text_data) => Some(
                        DatexExpressionData::Text(short_text_data.0)
                            .with_default_span(),
                    ),
                    RegularInstruction::Text(text_data) => Some(
                        DatexExpressionData::Text(text_data.0)
                            .with_default_span(),
                    ),
                    RegularInstruction::True => Some(
                        DatexExpressionData::Boolean(true).with_default_span(),
                    ),
                    RegularInstruction::False => Some(
                        DatexExpressionData::Boolean(false).with_default_span(),
                    ),
                    RegularInstruction::Null => {
                        Some(DatexExpressionData::Null.with_default_span())
                    }
                    RegularInstruction::Statements(statements_data)
                    | RegularInstruction::ShortStatements(statements_data) => {
                        let count = statements_data.statements_count;
                        collector.collect(
                            Instruction::RegularInstruction(RegularInstruction::Statements(statements_data)),
                            count,
                        );
                        None
                    }
                    RegularInstruction::UnboundedStatements => todo!(),
                    RegularInstruction::UnboundedStatementsEnd(_) => todo!(),
                    RegularInstruction::List(list_data)
                    | RegularInstruction::ShortList(list_data) => {
                        let count = list_data.element_count;
                        collector.collect(
                            Instruction::RegularInstruction(RegularInstruction::List(list_data)),
                            count,
                        );
                        None
                    }
                    RegularInstruction::Map(map_data)
                    | RegularInstruction::ShortMap(map_data) => {
                        let count = map_data.element_count * 2;
                        collector.collect(
                            Instruction::RegularInstruction(RegularInstruction::Map(map_data)),
                            count,
                        );
                        None
                    }
                    RegularInstruction::KeyValueDynamic => todo!(),
                    RegularInstruction::KeyValueShortText(text_data) => Some(
                        DatexExpressionData::Text(text_data.0)
                            .with_default_span(),
                    ),
                    RegularInstruction::Add
                    | RegularInstruction::Subtract
                    | RegularInstruction::Multiply
                    | RegularInstruction::Divide
                    | RegularInstruction::Matches
                    | RegularInstruction::StructuralEqual
                    | RegularInstruction::Equal
                    | RegularInstruction::NotStructuralEqual
                    | RegularInstruction::NotEqual
                    => {
                        collector.collect(
                            Instruction::RegularInstruction(
                                regular_instruction.clone(),
                            ),
                            2,
                        );
                        None
                    }
                    RegularInstruction::UnaryMinus
                    | RegularInstruction::UnaryPlus
                    | RegularInstruction::BitwiseNot
                    | RegularInstruction::CreateRef
                    | RegularInstruction::CreateRefMut
                    | RegularInstruction::Deref
                    => {
                        collector.collect(
                            Instruction::RegularInstruction(
                                regular_instruction.clone(),
                            ),
                            1,
                        );
                        None
                    }
                    RegularInstruction::Apply(_) => todo!(),
                    RegularInstruction::Is => todo!(),
                    RegularInstruction::AddAssign(_) => todo!(),
                    RegularInstruction::SubtractAssign(_) => todo!(),
                    RegularInstruction::MultiplyAssign(_) => todo!(),
                    RegularInstruction::DivideAssign(_) => todo!(),
                    RegularInstruction::GetRef(_) => todo!(),
                    RegularInstruction::GetLocalRef(_) => todo!(),
                    RegularInstruction::GetInternalRef(_) => todo!(),
                    RegularInstruction::GetOrCreateRef(_) => todo!(),
                    RegularInstruction::GetOrCreateRefMut(_) => todo!(),
                    RegularInstruction::AllocateSlot(_) => todo!(),
                    RegularInstruction::GetSlot(_) => todo!(),
                    RegularInstruction::DropSlot(_) => todo!(),
                    RegularInstruction::SetSlot(_) => todo!(),
                    RegularInstruction::AssignToReference(_) => todo!(),
                    RegularInstruction::TypedValue => None,
                    RegularInstruction::TypeExpression => todo!(),
                };

                if let Some(expr) = expr {
                    collector.push_result(expr);
                }

                // handle collecting nested expressions
                if let Some((instruction, mut collected_results)) =
                    collector.try_pop_collected()
                {
                    let expr = match instruction {
                        Instruction::RegularInstruction(
                            regular_instruction,
                        ) => match regular_instruction {
                            RegularInstruction::List(_)
                            | RegularInstruction::ShortList(_) => {
                                let elements = collected_results.collect_expressions();
                                DatexExpressionData::List(List::new(elements))
                                    .with_default_span()
                            }
                            RegularInstruction::Map(_)
                            | RegularInstruction::ShortMap(_) => {
                                let mut entries = Vec::new();
                                while !collected_results.is_empty() {
                                    entries.push(
                                        (
                                            collected_results.pop_expression(),
                                            collected_results.pop_expression(),
                                        )
                                    );
                                }
                                DatexExpressionData::Map(Map::new(entries))
                                    .with_default_span()
                            }
                            RegularInstruction::Statements(statements_data)
                            | RegularInstruction::ShortStatements(
                                statements_data,
                            ) => {
                                let statements = collected_results.collect_expressions();
                                DatexExpressionData::Statements(Statements {
                                    statements,
                                    is_terminated: statements_data.terminated,
                                    unbounded: None,
                                })
                                .with_default_span()
                            }

                            RegularInstruction::Add
                            | RegularInstruction::Subtract
                            | RegularInstruction::Multiply
                            | RegularInstruction::Divide
                            | RegularInstruction::Matches
                            | RegularInstruction::StructuralEqual
                            | RegularInstruction::Equal
                            | RegularInstruction::NotStructuralEqual
                            | RegularInstruction::NotEqual
                            => {
                                let right = collected_results.pop_expression();
                                let left = collected_results.pop_expression();
                                DatexExpressionData::BinaryOperation(BinaryOperation {
                                    operator: BinaryOperator::from(&regular_instruction),
                                    left: Box::new(left),
                                    right: Box::new(right),
                                    ty: None
                                }).with_default_span()
                            }
                            
                            RegularInstruction::UnaryMinus
                            | RegularInstruction::UnaryPlus
                            | RegularInstruction::BitwiseNot
                            | RegularInstruction::CreateRef
                            | RegularInstruction::CreateRefMut
                            | RegularInstruction::Deref
                            => {
                                let expr = collected_results.pop_expression();
                                DatexExpressionData::UnaryOperation(UnaryOperation {
                                    operator: UnaryOperator::from(&regular_instruction),
                                    expression: Box::new(expr),
                                }).with_default_span()
                            }

                            e => {
                                todo!(
                                    "Unhandled collected regular instruction: {:?}",
                                    e
                                );
                            }
                        },

                        Instruction::TypeInstruction(_) => {
                            todo!()
                        }
                    };
                    collector.push_result(expr);
                }
            }
            Instruction::TypeInstruction(type_instruction) => {
                let type_expression: Option<TypeExpression> =
                    match type_instruction {
                        TypeInstruction::List(list) => {
                            let count = list.element_count;
                            collector.collect(
                                Instruction::TypeInstruction(
                                    TypeInstruction::List(list),
                                ),
                                count,
                            );
                            None
                        }
                        TypeInstruction::ImplType(impl_type_data) => {
                            todo!("Handle TypeInstruction::ImplType")
                        }
                        TypeInstruction::LiteralInteger(integer_data) => Some(
                            TypeExpressionData::Integer(integer_data.0)
                                .with_default_span(),
                        ),
                        TypeInstruction::LiteralText(text_data) => Some(
                            TypeExpressionData::Text(text_data.0)
                                .with_default_span(),
                        ),
                        TypeInstruction::TypeReference(reference) => Some(
                            TypeExpressionData::GetReference(
                                PointerAddress::from(reference.address),
                            )
                            .with_default_span(),
                        ),

                        // TODO Handle different type instructions here
                        _ => todo!(),
                    };
                if let Some(type_expression) = type_expression {
                    collector.push_result(type_expression);
                }
            }
        }
    }

    collector
        .pop_datex_expression()
        .ok_or(DXBParserError::ExpectingMoreInstructions)
}

#[cfg(test)]
mod tests {
    use super::*;
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
            0x2A, // 42
            InstructionCode::UINT_8 as u8,
            0x15, // 21
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
}
