use crate::ast::structs::expression::DatexExpressionData;
use crate::ast::structs::r#type::TypeExpressionData;
use crate::global::protocol_structures::instructions::{Instruction, RegularInstruction, TypeInstruction};
use crate::values::core_values::decimal::Decimal;
use crate::values::core_values::decimal::typed_decimal::TypedDecimal;
use crate::values::core_values::integer::typed_integer::TypedInteger;
use crate::values::pointer::PointerAddress;

pub trait CollectionResultsPopper<R, V, K, T>: GetResults<R> {

    fn try_extract_value_result(result: R) -> Option<V>;
    fn try_extract_type_result(result: R) -> Option<T>;
    fn try_extract_key_value_pair_result(result: R) -> Option<(K, V)>;

    fn try_pop_value_result(&mut self) -> Option<V> {
        let result = self.pop()?;
        Self::try_extract_value_result(result)
    }
    fn try_pop_type_result(&mut self) -> Option<T> {
        let result = self.pop()?;
        Self::try_extract_type_result(result)
    }
    fn try_pop_key_value_pair_result(&mut self) -> Option<(K, V)> {
        let result = self.pop()?;
        Self::try_extract_key_value_pair_result(result)
    }

    fn pop_value_result(&mut self) -> V {
        self.try_pop_value_result().expect("Expected value result")
    }
    fn pop_type_result(&mut self) -> T {
        self.try_pop_type_result().expect("Expected type result")
    }
    fn pop_key_value_pair_result(&mut self) -> (K, V) {
        self.try_pop_key_value_pair_result()
            .expect("Expected key-value pair result")
    }

    fn pop(&mut self) -> Option<R> {
        self.get_results_mut().pop()
    }

    fn push(&mut self, result: R) {
        self.get_results_mut().push(result);
    }

    fn len(&self) -> usize {
        self.get_results().len()
    }

    fn is_empty(&self) -> bool {
        self.get_results().is_empty()
    }

    /// Collects all value results
    /// Panics if any of the popped results are not value results
    fn collect_value_results(&mut self) -> Vec<V> {
        let count = self.len();
        let mut expressions = Vec::with_capacity(count);
        for _ in 0..count {
            expressions.push(self.pop_value_result());
        }
        expressions.reverse();
        expressions
    }

    /// Collects all key-value pair results
    /// Panics if any of the popped results are not key-value pairs
    fn collect_key_value_pair_results(&mut self) -> Vec<(K, V)> {
        let count = self.len();
        let mut expression_pairs = Vec::with_capacity(count);
        for _ in 0..count {
            let pair = self.pop_key_value_pair_result();
            expression_pairs.push(pair);
        }
        expression_pairs.reverse();
        expression_pairs
    }

    /// Collects all type results
    /// Panics if any of the popped results are not type results
    fn collect_type_results(&mut self) -> Vec<T> {
        let count = self.len();
        let mut type_expressions = Vec::with_capacity(count);
        for _ in 0..count {
            type_expressions.push(self.pop_type_result());
        }
        type_expressions.reverse();
        type_expressions
    }
}

#[derive(Debug)]
pub struct CollectedResults<T> {
    results: Vec<T>,
}

impl<T> Default for CollectedResults<T> {
    fn default() -> Self {
        CollectedResults {
            results: Vec::new(),
        }
    }
}


trait GetResults<T> {
    fn get_results(&self) -> &Vec<T>;
    fn get_results_mut(&mut self) -> &mut Vec<T> ;
}

impl<T> GetResults<T> for CollectedResults<T> {
    fn get_results(&self) -> &Vec<T> {
        &self.results
    }
    fn get_results_mut(&mut self) -> &mut Vec<T> {
        &mut self.results
    }
}



#[derive(Debug)]
pub struct ResultCollector<T> {
    instruction: Option<Instruction>,
    count: usize,
    collected_results: CollectedResults<T>,
}

impl<T> Default for ResultCollector<T> {
    fn default() -> Self {
        ResultCollector {
            instruction: None,
            count: 1,
            collected_results: CollectedResults::default(),
        }
    }
}

impl<T> ResultCollector<T> {
    pub fn push_result(&mut self, result: impl Into<T>) {
        self.collected_results.get_results_mut().push(result.into());
    }

    pub fn try_pop_collected(
        &mut self,
    ) -> Option<(Instruction, CollectedResults<T>)> {
        if self.collected_results.get_results().len() == self.count {
            Some((self.instruction.take().unwrap(), core::mem::take(&mut self.collected_results)))
        } else if self.collected_results.get_results().len() > self.count {
            panic!(
                "Collected more results than expected for the last instruction"
            );
        } else {
            None
        }
    }
}


#[derive(Debug)]
pub struct InstructionCollector<T> {
    result_collectors: Vec<ResultCollector<T>>,
    root_result: Option<T>
}

impl<T> Default for InstructionCollector<T> {
    fn default() -> Self {
        InstructionCollector {
            result_collectors: Vec::new(),
            root_result: None,
        }
    }
}

impl<T> InstructionCollector<T> {
    pub fn collect(&mut self, instruction: Instruction, count: u32) {
        self.result_collectors.push(ResultCollector {
            instruction: Some(instruction),
            count: count as usize,
            collected_results: CollectedResults::default(),
        });
    }

    pub fn is_collecting(&self) -> bool {
        !self.result_collectors.is_empty()
    }

    pub fn push_result(&mut self, result: impl Into<T>) {
        let result = result.into();
        if let Some(result_collector) = self.result_collectors.last_mut() {
            result_collector.collected_results.get_results_mut().push(result);
        } else {
            self.root_result = Some(result);
        }
    }

    pub fn try_pop_collected(
        &mut self,
    ) -> Option<(Instruction, CollectedResults<T>)> {
        let result_collector = self.result_collectors.last_mut()?;
        let results = result_collector.try_pop_collected();
        if results.is_some() {
            self.result_collectors.pop();
        }
        results
    }

    pub fn take_root_result(&mut self) -> Option<T> {
        self.root_result.take()
    }

    /// Processes a regular instruction with default behavior for recursive instructions that need to
    /// collect more results.
    /// Returns Some(regular_instruction) if the instruction was not handled and should be processed by the caller.
    pub fn default_regular_instruction_collection(&mut self, regular_instruction: RegularInstruction) -> Option<RegularInstruction> {
        match regular_instruction {
            RegularInstruction::Statements(statements_data)
            | RegularInstruction::ShortStatements(statements_data) => {
                let count = statements_data.statements_count;
                self.collect(
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
                self.collect(
                    Instruction::RegularInstruction(RegularInstruction::List(list_data)),
                    count,
                );
                None
            }
            RegularInstruction::Map(map_data)
            | RegularInstruction::ShortMap(map_data) => {
                let count = map_data.element_count;
                self.collect(
                    Instruction::RegularInstruction(RegularInstruction::Map(map_data)),
                    count,
                );
                None
            }
            RegularInstruction::KeyValueDynamic => {
                self.collect(
                    Instruction::RegularInstruction(
                        regular_instruction.clone(),
                    ),
                    2,
                );
                None
            }
            RegularInstruction::KeyValueShortText(_) => {
                self.collect(
                    Instruction::RegularInstruction(
                        regular_instruction.clone(),
                    ),
                    1,
                );
                None
            },
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
                self.collect(
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
                self.collect(
                    Instruction::RegularInstruction(
                        regular_instruction.clone(),
                    ),
                    1,
                );
                None
            }
            RegularInstruction::TypedValue => {
                self.collect(
                    Instruction::RegularInstruction(
                        regular_instruction.clone(),
                    ),
                    2,
                );
                None
            },
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
            RegularInstruction::TypeExpression => todo!(),
            RegularInstruction::RemoteExecution(_) => todo!(),

            _ => Some(regular_instruction)
        }
    }

    /// Processes a type instruction with default behavior for recursive instructions that need to
    /// collect more results.
    /// Returns Some(type_instruction) if the instruction was not handled and should be processed by the caller.
    pub fn default_type_instruction_collection(&mut self, type_instruction: TypeInstruction) -> Option<TypeInstruction> {
        match type_instruction {
            TypeInstruction::List(list) => {
                let count = list.element_count;
                self.collect(
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

            _ => Some(type_instruction)
        }
    }

}
