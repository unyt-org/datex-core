use crate::global::protocol_structures::instructions::{
    Instruction, RegularInstruction, TypeInstruction,
};
use crate::stdlib::vec::Vec;

pub trait CollectionResultsPopper<R, V, K, T>: GetResults<R> + Sized {
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
    fn collect_value_results(mut self) -> Vec<V> {
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
    fn collect_key_value_pair_results(mut self) -> Vec<(K, V)> {
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
    fn collect_type_results(mut self) -> Vec<T> {
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
    fn get_results_mut(&mut self) -> &mut Vec<T>;
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
pub enum ResultCollector<T> {
    Full(FullResultCollector<T>),
    Last(LastResultCollector<T>),
    FullUnbounded(FullUnboundedResultCollector<T>),
    LastUnbounded(LastUnboundedResultCollector<T>),
}

pub enum FullOrPartialResult<T> {
    Full(Instruction, CollectedResults<T>),
    Partial(Instruction, Option<T>),
}

#[derive(Debug)]
pub struct FullResultCollector<T> {
    instruction: Option<Instruction>,
    expected_count: u32,
    collected_results: CollectedResults<T>,
}

#[derive(Debug)]
pub struct LastResultCollector<T> {
    instruction: Option<Instruction>,
    expected_count: u32,
    collected_count: u32,
    last_result: Option<T>,
}

#[derive(Debug)]
pub struct FullUnboundedResultCollector<T> {
    instruction: Option<Instruction>,
    collected_results: CollectedResults<T>,
}

#[derive(Debug)]
pub struct LastUnboundedResultCollector<T> {
    instruction: Option<Instruction>,
    pub(crate) last_result: Option<T>,
}

impl<T> ResultCollector<T> {
    pub fn push_result(&mut self, result: impl Into<T>) {
        match self {
            ResultCollector::Full(collector) => {
                if collector.collected_results.get_results().len() as u32
                    >= collector.expected_count
                {
                    panic!(
                        "Collected more results than expected for the instruction"
                    );
                }
                collector
                    .collected_results
                    .get_results_mut()
                    .push(result.into());
            }
            ResultCollector::Last(collector) => {
                if collector.collected_count >= collector.expected_count {
                    panic!(
                        "Collected more results than expected for the instruction"
                    );
                }
                collector.last_result = Some(result.into());
                collector.collected_count += 1;
            }
            ResultCollector::FullUnbounded(collector) => {
                collector
                    .collected_results
                    .get_results_mut()
                    .push(result.into());
            }
            ResultCollector::LastUnbounded(collector) => {
                collector.last_result = Some(result.into());
            }
        }
    }

    pub fn try_pop_collected(&mut self) -> Option<FullOrPartialResult<T>> {
        match self {
            ResultCollector::Full(collector) => {
                if collector.collected_results.get_results().len() as u32
                    == collector.expected_count
                {
                    Some(FullOrPartialResult::Full(
                        collector.instruction.take().unwrap(),
                        core::mem::take(&mut collector.collected_results),
                    ))
                } else if collector.collected_results.get_results().len() as u32
                    > collector.expected_count
                {
                    panic!(
                        "Collected more results than expected for the last instruction"
                    );
                } else {
                    None
                }
            }
            ResultCollector::Last(collector) => {
                if collector.collected_count == collector.expected_count {
                    Some(FullOrPartialResult::Partial(
                        collector.instruction.take().unwrap(),
                        collector.last_result.take(),
                    ))
                } else if collector.collected_count > collector.expected_count {
                    panic!(
                        "Collected more results than expected for the last instruction"
                    );
                } else {
                    None
                }
            }
            // unbounded results must be explicitly popped with try_pop_unbounded
            ResultCollector::LastUnbounded(_) => None,
            ResultCollector::FullUnbounded(_) => None,
        }
    }

    pub fn try_pop_unbounded(&mut self) -> Option<FullOrPartialResult<T>> {
        match self {
            ResultCollector::LastUnbounded(collector) => {
                Some(FullOrPartialResult::Partial(
                    collector.instruction.take().unwrap(),
                    collector.last_result.take(),
                ))
            }
            ResultCollector::FullUnbounded(collector) => {
                Some(FullOrPartialResult::Full(
                    collector.instruction.take().unwrap(),
                    core::mem::take(&mut collector.collected_results),
                ))
            }
            _ => None,
        }
    }
}

#[derive(Debug)]
pub struct InstructionCollector<T> {
    result_collectors: Vec<ResultCollector<T>>,
    root_result: Option<T>,
}

impl<T> Default for InstructionCollector<T> {
    fn default() -> Self {
        InstructionCollector {
            result_collectors: Vec::new(),
            root_result: None,
        }
    }
}

#[derive(Debug)]
pub enum StatementResultCollectionStrategy {
    Full,
    Last,
}

impl<T> InstructionCollector<T> {
    pub fn collect_full(
        &mut self,
        instruction: Instruction,
        expected_count: u32,
    ) {
        self.result_collectors.push(ResultCollector::Full(
            FullResultCollector {
                instruction: Some(instruction),
                expected_count,
                collected_results: CollectedResults::default(),
            },
        ));
    }

    pub fn collect_last(
        &mut self,
        instruction: Instruction,
        expected_count: u32,
    ) {
        self.result_collectors.push(ResultCollector::Last(
            LastResultCollector {
                instruction: Some(instruction),
                expected_count,
                collected_count: 0,
                last_result: None,
            },
        ));
    }

    pub fn collect_full_unbounded(&mut self, instruction: Instruction) {
        self.result_collectors.push(ResultCollector::FullUnbounded(
            FullUnboundedResultCollector {
                instruction: Some(instruction),
                collected_results: CollectedResults::default(),
            },
        ));
    }

    pub fn collect_last_unbounded(&mut self, instruction: Instruction) {
        self.result_collectors.push(ResultCollector::LastUnbounded(
            LastUnboundedResultCollector {
                instruction: Some(instruction),
                last_result: None,
            },
        ));
    }

    pub fn is_collecting(&self) -> bool {
        !self.result_collectors.is_empty()
    }

    pub fn push_result(&mut self, result: impl Into<T>) {
        let result = result.into();
        if let Some(result_collector) = self.result_collectors.last_mut() {
            result_collector.push_result(result);
        } else {
            self.root_result = Some(result);
        }
    }

    pub fn try_pop_collected(&mut self) -> Option<FullOrPartialResult<T>> {
        let result_collector = self.result_collectors.last_mut()?;
        let results = result_collector.try_pop_collected();
        if results.is_some() {
            self.result_collectors.pop();
        }
        results
    }

    pub fn try_pop_unbounded(&mut self) -> Option<FullOrPartialResult<T>> {
        let result_collector = self.result_collectors.last_mut()?;
        let results = result_collector.try_pop_unbounded();
        if results.is_some() {
            self.result_collectors.pop();
        }
        results
    }

    pub fn last(&self) -> Option<&ResultCollector<T>> {
        self.result_collectors.last()
    }

    pub fn take_root_result(&mut self) -> Option<T> {
        self.root_result.take()
    }

    /// Processes a regular instruction with default behavior for recursive instructions that need to
    /// collect more results.
    /// Returns Some(regular_instruction) if the instruction was not handled and should be processed by the caller.
    pub fn default_regular_instruction_collection(
        &mut self,
        regular_instruction: RegularInstruction,
        statement_result_collection_strategy: StatementResultCollectionStrategy,
    ) -> Option<RegularInstruction> {
        match regular_instruction {
            RegularInstruction::Statements(statements_data)
            | RegularInstruction::ShortStatements(statements_data) => {
                let count = statements_data.statements_count;
                match statement_result_collection_strategy {
                    StatementResultCollectionStrategy::Full => {
                        self.collect_full(
                            Instruction::RegularInstruction(
                                RegularInstruction::Statements(statements_data),
                            ),
                            count,
                        );
                    }
                    StatementResultCollectionStrategy::Last => {
                        self.collect_last(
                            Instruction::RegularInstruction(
                                RegularInstruction::Statements(statements_data),
                            ),
                            count,
                        );
                    }
                }

                None
            }
            RegularInstruction::UnboundedStatements => {
                match statement_result_collection_strategy {
                    StatementResultCollectionStrategy::Full => {
                        self.collect_full_unbounded(
                            Instruction::RegularInstruction(
                                RegularInstruction::UnboundedStatements,
                            ),
                        );
                    }
                    StatementResultCollectionStrategy::Last => {
                        self.collect_last_unbounded(
                            Instruction::RegularInstruction(
                                RegularInstruction::UnboundedStatements,
                            ),
                        );
                    }
                }
                None
            }
            RegularInstruction::UnboundedStatementsEnd(statements_end) => {
                self.collect_full(
                    Instruction::RegularInstruction(
                        RegularInstruction::UnboundedStatementsEnd(
                            statements_end,
                        ),
                    ),
                    0,
                );
                None
            }
            RegularInstruction::List(list_data)
            | RegularInstruction::ShortList(list_data) => {
                let count = list_data.element_count;
                self.collect_full(
                    Instruction::RegularInstruction(RegularInstruction::List(
                        list_data,
                    )),
                    count,
                );
                None
            }
            RegularInstruction::Map(map_data)
            | RegularInstruction::ShortMap(map_data) => {
                let count = map_data.element_count;
                self.collect_full(
                    Instruction::RegularInstruction(RegularInstruction::Map(
                        map_data,
                    )),
                    count,
                );
                None
            }
            RegularInstruction::KeyValueDynamic => {
                self.collect_full(
                    Instruction::RegularInstruction(regular_instruction),
                    2,
                );
                None
            }
            RegularInstruction::KeyValueShortText(_) => {
                self.collect_full(
                    Instruction::RegularInstruction(regular_instruction),
                    1,
                );
                None
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
            | RegularInstruction::Is => {
                self.collect_full(
                    Instruction::RegularInstruction(regular_instruction),
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
            | RegularInstruction::GetOrCreateRef(_)
            | RegularInstruction::GetOrCreateRefMut(_) => {
                self.collect_full(
                    Instruction::RegularInstruction(regular_instruction),
                    1,
                );
                None
            }
            RegularInstruction::TypedValue => {
                self.collect_full(
                    Instruction::RegularInstruction(regular_instruction),
                    2,
                );
                None
            }

            RegularInstruction::SetSlot(_)
            | RegularInstruction::AllocateSlot(_)
            | RegularInstruction::AddAssign(_)
            | RegularInstruction::SubtractAssign(_)
            | RegularInstruction::MultiplyAssign(_)
            | RegularInstruction::DivideAssign(_) => {
                self.collect_full(
                    Instruction::RegularInstruction(regular_instruction),
                    1,
                );
                None
            }

            RegularInstruction::SetReferenceValue(_) => {
                self.collect_full(
                    Instruction::RegularInstruction(regular_instruction),
                    2,
                );
                None
            }

            RegularInstruction::TypeExpression => {
                self.collect_full(
                    Instruction::RegularInstruction(regular_instruction),
                    1,
                );
                None
            }

            RegularInstruction::RemoteExecution(_) => {
                self.collect_full(
                    Instruction::RegularInstruction(regular_instruction),
                    1,
                );
                None
            }

            RegularInstruction::Apply(apply_data) => {
                let count = apply_data.arg_count as u32;
                self.collect_full(
                    Instruction::RegularInstruction(RegularInstruction::Apply(
                        apply_data,
                    )),
                    count + 1,
                );
                None
            }

            _ => Some(regular_instruction),
        }
    }

    /// Processes a type instruction with default behavior for recursive instructions that need to
    /// collect more results.
    /// Returns Some(type_instruction) if the instruction was not handled and should be processed by the caller.
    pub fn default_type_instruction_collection(
        &mut self,
        type_instruction: TypeInstruction,
    ) -> Option<TypeInstruction> {
        match type_instruction {
            TypeInstruction::List(list) => {
                let count = list.element_count;
                self.collect_full(
                    Instruction::TypeInstruction(TypeInstruction::List(list)),
                    count,
                );
                None
            }
            TypeInstruction::ImplType(impl_type_data) => {
                self.collect_full(
                    Instruction::TypeInstruction(TypeInstruction::ImplType(
                        impl_type_data,
                    )),
                    1,
                );
                None
            }

            _ => Some(type_instruction),
        }
    }
}
