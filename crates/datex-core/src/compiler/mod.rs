//! This module contains the main compiler logic for DATEX, precompilation, and compilation to DXB bytecode.
use crate::{
    ast::{
        expressions::{
            BinaryOperation, CallableDeclaration, ComparisonOperation,
            Conditional, DatexExpression, DatexExpressionData,
            PropertyAssignment, RemoteExecution, RootPropertyAccess,
            Statements, UnaryOperation, UnboundedStatement, UnboxAssignment,
            ValueAccessType, VariableAccess, VariableAssignment,
            VariableDeclaration, VariableKind,
        },
        resolved_variable::VariableId,
    },
    compiler::{
        compilation_result::CompilationResult,
        context::CompilationContext,
        error::{
            CompilerError, DetailedCompilerErrorsWithMaybeRichAst,
            SimpleCompilerErrorOrDetailedCompilerErrorWithRichAst,
            SimpleOrDetailedCompilerError, SpannedCompilerError,
        },
        metadata::CompileContextMetadata,
        scope::CompilationScope,
    },
    core_compiler::{
        core_compilation_context::{CompileInput, DXBWithSharedValues},
        to_instructions::ToInstructions,
    },
    global::{
        dxb_block::DXBBlock,
        operators::modification::ModificationOperator,
        protocol_structures::{
            block_header::BlockHeader,
            encrypted_header::EncryptedHeader,
            injected_values::{
                InjectedValueType, LocalInjectedValueType,
                SharedInjectedValueType,
            },
            routing_header::RoutingHeader,
        },
        root_properties::RootProperty,
        stack_index::StackIndex,
    },
    instruction::{
        Instruction,
        instruction_codes::InstructionCode,
        instruction_data::{
            CallableDeclarationData, CallableSignatureData,
            InstructionBlockData, JumpData, ShortTextData,
            UnboundedStatementsData,
        },
        regular_instruction::RegularInstruction,
    },
    parser::{Parser, ParserOptions, errors::SpannedParserError},
    prelude::*,
    runtime::{Runtime, execution::context::ExecutionMode},
    shared_values::{
        ReferenceMutability, SharedContainer, SharedContainerMutability,
        traits::SharedContainerCommon,
    },
    time::Instant as TimingInstant,
    utils::buffers::{append_u8, append_u32},
    values::{
        core_values::endpoint::Endpoint, value_container::ValueContainer,
    },
};
use core::str::FromStr;
use log::{debug, info};
use precompiler::{
    options::PrecompilerOptions,
    precompile_ast,
    precompiled_ast::{AstMetadata, RichAst, VariableMetadata},
};

pub mod ast_node_to_instructions;
pub mod compilation_result;
pub mod context;
pub mod error;
pub mod metadata;
pub mod precompiler;
pub mod scope;
pub mod type_compiler;
#[cfg(feature = "std")]
pub mod workspace;

#[derive(Clone, Default)]
pub struct CompileOptions {
    pub compile_scope: CompilationScope,
    pub parser_options: ParserOptions,

    /// If the receiver of a compilation is known (e.g. remote execution), we can pass that value
    pub receivers: Vec<Endpoint>,
}

impl CompileOptions {
    pub fn new(
        compile_scope: CompilationScope,
        receivers: Vec<Endpoint>,
    ) -> Self {
        CompileOptions {
            compile_scope,
            parser_options: ParserOptions::default(),
            receivers,
        }
    }
}

#[derive(Debug)]
pub enum StaticValueOrDXB {
    StaticValue(Option<ValueContainer>),
    DXB(DXBWithSharedValues),
}

#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub enum VariableModel {
    /// A variable that is declared once and never reassigned afterward
    /// e.g. `const a = 42;`
    Constant,
    /// A variable that can be reassigned by updating the slot value
    /// e.g. `var a = 42; a = 69;`
    VariableSlot,
}

impl From<VariableRepresentation> for VariableModel {
    fn from(value: VariableRepresentation) -> Self {
        match value {
            VariableRepresentation::Constant => VariableModel::Constant,
            VariableRepresentation::VariableSlot => VariableModel::VariableSlot,
        }
    }
}

impl VariableModel {
    /// Determines the variable model based on the variable kind and metadata.
    pub fn infer(
        variable_kind: &VariableKind,
        _variable_metadata: Option<VariableMetadata>,
        _execution_mode: ExecutionMode,
    ) -> Self {
        // const variables are always constant
        if *variable_kind == VariableKind::Const {
            VariableModel::Constant
        }
        // otherwise, we use VariableSlot (default for `var` variables)
        else {
            VariableModel::VariableSlot
        }
    }

    pub fn infer_from_ast_metadata_and_type(
        ast_metadata: &AstMetadata,
        variable_id: &Option<VariableId>,
        variable_kind: &VariableKind,
        execution_mode: ExecutionMode,
    ) -> Self {
        let variable_metadata =
            variable_id.and_then(|id| ast_metadata.variable_metadata(id));
        Self::infer(variable_kind, variable_metadata.cloned(), execution_mode)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub enum VariableRepresentation {
    Constant,
    VariableSlot,
}

/// Represents a variable in the DATEX script.
#[derive(Debug, Clone)]
pub struct Variable {
    pub name: String,
    pub kind: VariableKind,
    pub index: StackIndex,
    pub representation: VariableRepresentation,
}

impl Variable {
    pub fn new_const(name: String, index: StackIndex) -> Self {
        Variable {
            name,
            kind: VariableKind::Const,
            index,
            representation: VariableRepresentation::Constant,
        }
    }

    pub fn new_variable_slot(
        name: String,
        kind: VariableKind,
        index: StackIndex,
    ) -> Self {
        Variable {
            name,
            kind,
            index,
            representation: VariableRepresentation::VariableSlot,
        }
    }
}

/// Compiles a DATEX script text into a single DXB block including routing and block headers.
/// This function is used to create a block that can be sent over the network.
pub fn compile_block(
    datex_script: &str,
    runtime: Runtime,
) -> Result<Vec<u8>, SimpleOrDetailedCompilerError> {
    let (body, _) =
        compile_script(datex_script, CompileOptions::default(), runtime)?;

    let routing_header = RoutingHeader::default();

    let block_header = BlockHeader::default();
    let encrypted_header = EncryptedHeader::default();

    let block =
        DXBBlock::new(routing_header, block_header, encrypted_header, body);

    let bytes = block.to_bytes();
    Ok(bytes)
}

/// Compiles a DATEX script text into a DXB body
/// As there can be no potential shared values injected, and we plan to resolve a literal
/// pointer id only to execution, no shared values are returned.
pub fn compile_script(
    datex_script: &str,
    options: CompileOptions,
    runtime: Runtime,
) -> Result<(Vec<u8>, CompilationScope), SpannedCompilerError> {
    compile_template(datex_script, vec![], options, runtime)
        .map(|e| (e.dxb_with_shared_values.dxb, e.scope))
}

/// Directly extracts a static value from a DATEX script as a `ValueContainer`.
/// This only works if the script does not contain any dynamic values or operations.
/// All JSON-files can be compiled to static values, but not all DATEX scripts.
pub fn extract_static_value_from_script(
    datex_script: &str,
) -> Result<Option<ValueContainer>, SpannedParserError> {
    let valid_parse_result = Parser::parse_with_default_options(datex_script)?;
    Ok(ValueContainer::try_from(valid_parse_result.data()).ok())
}

/// Converts a DATEX script template text with inserted values into an AST with metadata
/// If the script does not contain any dynamic values or operations, the static result value is
/// directly returned instead of the AST.
pub fn compile_script_or_return_static_value(
    datex_script: &str,
    mut options: CompileOptions,
    runtime: Runtime,
) -> Result<(StaticValueOrDXB, CompilationScope), SpannedCompilerError> {
    let ast = parse_datex_script_to_rich_ast_simple_error(
        datex_script,
        &mut options,
        runtime.clone(),
    )?;
    let lookup = runtime.pointer_availability_lookup();
    let compilation_context = CompilationContext::new(
        vec![],
        options.compile_scope.execution_mode,
        CompileInput::new(&lookup, &options.receivers),
        options.compile_scope,
    );
    let compilation_result = compilation_context
        .compile_ast_node_to_dxb_with_shared_values(
            &*ast.ast.data,
            &mut ast.metadata.borrow_mut(),
        )?;
    if compilation_result.has_non_static_value {
        Ok((
            StaticValueOrDXB::DXB(compilation_result.dxb_with_shared_values),
            compilation_result.scope,
        ))
    } else {
        // try to extract static value from AST, otherwise panic (should not happen if compilation_result.has_non_static_value is false)
        let value_container = ValueContainer::try_from(ast.ast.data())
            .expect("AST should be a static value if compilation_result.has_non_static_value is false");

        Ok((
            StaticValueOrDXB::StaticValue(Some(value_container)),
            compilation_result.scope,
        ))
    }
}

/// Ensure that the root ast node is a statements node
/// Returns if the initial ast was terminated
fn ensure_statements(
    ast: &mut DatexExpression,
    unbounded_section: Option<UnboundedStatement>,
) -> bool {
    if let DatexExpressionData::Statements(Statements {
        is_terminated,
        unbounded,
        ..
    }) = ast.data_mut()
    {
        *unbounded = unbounded_section;
        *is_terminated
    } else {
        // wrap in statements
        let original_ast = ast.clone();
        *ast.data_mut() = DatexExpressionData::Statements(Statements {
            statements: vec![original_ast],
            is_terminated: false,
            unbounded: unbounded_section,
        });
        false
    }
}

/// Parses and precompiles a DATEX script template text with inserted values into an AST with metadata
/// Only returns the first occurring error
pub fn parse_datex_script_to_rich_ast_simple_error(
    datex_script: &str,
    options: &mut CompileOptions,
    runtime: Runtime,
) -> Result<RichAst, SpannedCompilerError> {
    // TODO #481: do this (somewhere else)
    // // shortcut if datex_script is "?" - call compile_value_container directly
    // if datex_script == "?" {
    //     if inserted_values.len() != 1 {
    //         return Err(CompilerError::InvalidPlaceholderCount);
    //     }
    //     let result =
    //         compile_value_container(inserted_values[0]).map(StaticValueOrAst::from)?;
    //     return Ok((result, options.compile_scope));
    // }
    let parse_start = TimingInstant::now();
    let mut valid_parse_result =
        Parser::parse(datex_script, options.parser_options.clone())?;

    // make sure to append a statements block for the first block in ExecutionMode::Unbounded
    let is_terminated = if let ExecutionMode::Unbounded { has_next } =
        options.compile_scope.execution_mode
    {
        ensure_statements(
            &mut valid_parse_result,
            Some(UnboundedStatement {
                is_first: !options.compile_scope.was_used,
                is_last: !has_next,
            }),
        )
    } else {
        matches!(
            valid_parse_result.data(),
            &DatexExpressionData::Statements(Statements {
                is_terminated: true,
                ..
            })
        )
    };

    debug!(" [parse took {} ms]", parse_start.elapsed().as_millis());
    let precompile_start = TimingInstant::now();
    let res = precompile_to_rich_ast(
        valid_parse_result,
        &mut options.compile_scope,
        PrecompilerOptions {
            detailed_errors: false,
        },
        runtime,
    )
    .map_err(|e| match e {
        SimpleCompilerErrorOrDetailedCompilerErrorWithRichAst::Simple(e) => e,
        _ => unreachable!(), // because detailed_errors: false
    })
    .inspect(|ast| {
        // store information about termination (last semicolon) in metadata
        ast.metadata.borrow_mut().is_terminated = is_terminated;
    });
    debug!(
        " [precompile took {} ms]",
        precompile_start.elapsed().as_millis()
    );
    res
}

/// Parses and precompiles a DATEX script template text with inserted values into an AST with metadata
/// Returns all occurring errors and the AST if one or more errors occur.
pub fn parse_datex_script_to_rich_ast_detailed_errors(
    datex_script: &str,
    options: &mut CompileOptions,
    runtime: Runtime,
) -> Result<RichAst, DetailedCompilerErrorsWithMaybeRichAst> {
    let (ast, parser_errors) =
        Parser::parse_collecting_with_default_options(datex_script)
            .into_ast_and_errors();
    precompile_to_rich_ast(
        ast,
        &mut options.compile_scope,
        PrecompilerOptions {
            detailed_errors: true,
        },
        runtime,
    )
    .map_err(|e| match e {
        SimpleCompilerErrorOrDetailedCompilerErrorWithRichAst::Detailed(
            mut e,
        ) => {
            // append parser errors to detailed errors
            e.errors.errors.extend(
                parser_errors.into_iter().map(SpannedCompilerError::from),
            );
            e.into()
        }
        _ => unreachable!(), // because detailed_errors: true
    })
}

/// Compiles a DATEX script template text with inserted values into a DXB body
pub fn compile_template(
    datex_script: &str,
    inserted_values: Vec<Option<ValueContainer>>,
    mut options: CompileOptions,
    runtime: Runtime,
) -> Result<CompilationResult, SpannedCompilerError> {
    let ast = parse_datex_script_to_rich_ast_simple_error(
        datex_script,
        &mut options,
        runtime.clone(),
    )?;

    let lookup = runtime.pointer_availability_lookup();
    let input = CompileInput::new(&lookup, &options.receivers);
    let compilation_context = CompilationContext::new(
        inserted_values,
        options.compile_scope.execution_mode,
        input,
        options.compile_scope,
    );
    let compile_start = TimingInstant::now();

    let res = compilation_context
        .compile_ast_with_metadata_to_dxb_with_shared_values(ast)
        .map_err(SpannedCompilerError::from);
    
    debug!(
        " [compile_ast took {} ms]",
        compile_start.elapsed().as_millis()
    );
    res
}

/// Macro for compiling a DATEX script template text with inserted values into a DXB body,
/// behaves like the format! macro.
/// Example:
/// ```
/// use datex_core::{compile, runtime::Runtime};
/// let runtime: Runtime;
///
/// # runtime = Runtime::stub();
/// compile!(runtime, "4 + ?", 42);
/// compile!(runtime, "? + ?", 1, 2);
/// ```
#[macro_export]
macro_rules! compile {
    ($runtime:expr, $fmt:literal $(, $arg:expr )* $(,)?) => {
        {
            let script: &str = $fmt.into();
            let values: Vec<Option<$crate::values::value_container::ValueContainer>> = vec![$(Some($arg.into())),*];

            $crate::compiler::compile_template(&script, values, $crate::compiler::CompileOptions::default(), $runtime.clone())
        }
    }
}

/// Precompiles a DATEX expression AST into an AST with metadata.
fn precompile_to_rich_ast(
    valid_parse_result: DatexExpression,
    scope: &mut CompilationScope,
    precompiler_options: PrecompilerOptions,
    runtime: Runtime,
) -> Result<RichAst, SimpleCompilerErrorOrDetailedCompilerErrorWithRichAst> {
    // if static execution mode and scope already used, return error
    if scope.execution_mode == ExecutionMode::Static && scope.was_used {
        return Err(
            SimpleCompilerErrorOrDetailedCompilerErrorWithRichAst::Simple(
                SpannedCompilerError::from(
                    CompilerError::OnceScopeUsedMultipleTimes,
                ),
            ),
        );
    }

    // set was_used to true
    scope.was_used = true;

    let rich_ast = if let Some(precompiler_data) = &scope.precompiler_data {
        // precompile the AST, adding metadata for variables etc.
        precompile_ast(
            valid_parse_result,
            &mut precompiler_data.precompiler_scope_stack.borrow_mut(),
            precompiler_data.rich_ast.metadata.clone(),
            precompiler_options,
            runtime,
        )?
    } else {
        // if no precompiler data, just use the AST with default metadata
        RichAst::new_without_metadata(valid_parse_result)
    };

    Ok(rich_ast)
}

pub fn compile_child_realm_instructions(
    compilation_context: &mut CompilationContext,
    ast: &DatexExpression,
    injected_variable_count: u32,
    metadata: &mut AstMetadata,
    existing_variables: Vec<Variable>,
) -> Result<(InstructionBlockData, CompilationScope), CompilerError> {
    let input = compilation_context.core_context.input.clone();
    // compile remote execution block

    let stack_index_offset =
        StackIndex(injected_variable_count + existing_variables.len() as u32);

    let injected_values_offset = StackIndex(existing_variables.len() as u32);

    let mut child_scope = CompilationScope::new_with_external_parent_scope(
        compilation_context.scope.clone(),
        stack_index_offset,
        injected_values_offset,
    );
    for variable in existing_variables {
        child_scope.register_variable_slot(variable);
    }

    let execution_block_ctx = CompilationContext::new(
        vec![],
        ExecutionMode::Static,
        input,
        child_scope,
    );

    // TODO: what to do with the shared values? [shared_values]
    let CompilationResult {
        dxb_with_shared_values:
            DXBWithSharedValues {
                dxb,
                shared_values: _,
            },
        scope: child_scope,
        ..
    } = execution_block_ctx
        .compile_ast_node_to_dxb_with_shared_values(&*ast.data, metadata)?;

    // reset to current scope
    let external_parent_scope = child_scope
        .pop_external()
        .ok_or(CompilerError::ScopePopError)?;

    // insert remote execution instruction
    Ok((
        InstructionBlockData {
            // block size (len of compilation_context.buffer)
            length: dxb.len() as u32,
            injected_value_count: external_parent_scope.injected_values.len()
                as u32,
            injected_values: external_parent_scope.injected_values,
            body: dxb,
        },
        *external_parent_scope.scope,
    ))
}

#[cfg(test)]
#[cfg(feature = "disassembler")]
pub mod tests {
    use super::{
        CompilationContext, CompileOptions, StaticValueOrDXB, compile_script,
        compile_script_or_return_static_value, compile_template,
        parse_datex_script_to_rich_ast_simple_error,
    };
    use crate::{
        ast::expressions::{CallableDeclaration, CallableSignature},
        compiler::scope::CompilationScope,
        core_compiler::core_compilation_context::{
            CompileInput, DXBWithSharedValues, default_compile_input,
        },
        instruction::{
            instruction_codes::InstructionCode,
            instruction_data::{
                CallableDeclarationData, CallableSignatureData,
            },
            regular_instruction::RegularInstruction,
            type_instruction::TypeInstruction,
        },
        runtime::execution::context::ExecutionMode,
        types::{
            literal_type_definition::LiteralTypeDefinition,
            type_definition::callable::CallableKind,
        },
        values::value_container::ValueContainer,
    };

    use crate::instruction::instruction_data::{
        CallableDeclarationDataDebugTree, InstructionBlockDataDebugFlat,
    };

    use crate::{
        compiler::{
            compilation_result::CompilationResult, error::CompilerError,
        },
        disassembler::{
            assertions::{
                assert_instructions_equal, instructions, instructions_with_span,
            },
            print_disassembled,
        },
        global::{
            protocol_structures::injected_values::{
                InjectedValueDeclaration, InjectedValueType,
                LocalInjectedValueType, SharedInjectedValueType,
            },
            root_properties::RootProperty,
            stack_index::StackIndex,
        },
        instruction::{
            Instruction,
            instruction_data::{
                InstructionBlockData, InstructionBlockDataDebugTree, ListData,
                MapData, ShortListData, ShortMapData, ShortTextData,
                StatementsData, TaggedValue, UInt8Data,
            },
        },
        libs::core::{
            core_lib_id::CoreLibId,
            type_id::{CoreLibBaseTypeId, CoreLibTypeId},
        },
        prelude::*,
        runtime::{Runtime, RuntimeConfig, RuntimeRunner},
        shared_values::PointerAddress,
        traits::convert_value_container::ConvertValueContainer,
        values::core_values::integer::{Integer, typed_integer::TypedInteger},
    };
    use alloc::format;
    use core::assert_matches;
    use log::*;

    fn compile_unwrap(script: &str) -> Vec<u8> {
        compile_script(script, CompileOptions::default(), Runtime::stub())
            .unwrap()
            .0
    }

    fn compile_and_log(datex_script: &str) -> Vec<u8> {
        let (result, _) = compile_script(
            datex_script,
            CompileOptions::default(),
            Runtime::stub(),
        )
        .unwrap();
        info!(
            "{:?}",
            result
                .iter()
                .map(|x| InstructionCode::try_from(*x).map(|x| x.to_string()))
                .map(|x| x.unwrap_or_else(|_| "Unknown".to_string()))
                .collect::<Vec<_>>()
        );
        result
    }

    fn get_compilation_result(script: &'_ str) -> CompilationResult {
        let mut options = CompileOptions::default();
        let ast = parse_datex_script_to_rich_ast_simple_error(
            script,
            &mut options,
            Runtime::stub(),
        )
        .unwrap();
        let input = unsafe { default_compile_input() };
        let mut compilation_context = CompilationContext::new(
            vec![],
            options.compile_scope.execution_mode,
            input,
            options.compile_scope,
        );
        compilation_context
            .compile_ast_with_metadata_to_dxb_with_shared_values(ast)
            .unwrap()
    }

    fn compile_datex_script_debug_unbounded(
        datex_script_parts: impl Iterator<Item = &'static str>,
    ) -> impl Iterator<Item = Vec<u8>> {
        let datex_script_parts = datex_script_parts.collect::<Vec<_>>();
        gen move {
            let mut compilation_scope =
                CompilationScope::new(ExecutionMode::unbounded());
            let len = datex_script_parts.len();
            for (index, script_part) in
                datex_script_parts.into_iter().enumerate()
            {
                // if last part, compile and return static value if possible
                if index == len - 1 {
                    compilation_scope.mark_as_last_execution();
                }
                let (result, new_compilation_scope) = compile_script(
                    script_part,
                    CompileOptions::new(compilation_scope, vec![]),
                    Runtime::stub(),
                )
                .unwrap();
                compilation_scope = new_compilation_scope;
                yield result;
            }
        }
    }

    fn assert_unbounded_input_matches_output(
        input: Vec<&'static str>,
        expected_output: Vec<Vec<u8>>,
    ) {
        let input = input.into_iter();
        let expected_output = expected_output.into_iter();
        for (result, expected) in
            compile_datex_script_debug_unbounded(input.into_iter())
                .zip(expected_output.into_iter())
        {
            assert_eq!(result, expected);
        }
    }

    #[test]
    fn simple_multiplication() {
        let lhs: u8 = 1;
        let rhs: u8 = 2;
        let datex_script = format!("{lhs}u8 * {rhs}u8"); // 1 * 2
        let result = compile_and_log(&datex_script);
        assert_eq!(
            result,
            vec![
                InstructionCode::MULTIPLY.into(),
                InstructionCode::UINT_8.into(),
                lhs,
                InstructionCode::UINT_8.into(),
                rhs,
            ]
        );
    }

    #[test]
    fn simple_multiplication_close() {
        let lhs: u8 = 1;
        let rhs: u8 = 2;
        let datex_script = format!("{lhs}u8 * {rhs}u8;"); // 1 * 2
        let result = compile_and_log(&datex_script);
        assert_eq!(
            result,
            vec![
                InstructionCode::SHORT_STATEMENTS.into(),
                1,
                1, // terminated
                InstructionCode::MULTIPLY.into(),
                InstructionCode::UINT_8.into(),
                lhs,
                InstructionCode::UINT_8.into(),
                rhs,
            ]
        );
    }

    #[test]
    fn shared_value() {
        let datex_script = "shared null";
        let result = compile_and_log(datex_script);
        assert_instructions_equal!(
            &result,
            (RegularInstruction::CreateShared, RegularInstruction::Null)
        );
    }

    #[test]
    fn shared_mut_value() {
        let datex_script = "shared mut null";
        let result = compile_and_log(datex_script);
        assert_instructions_equal!(
            &result,
            (
                RegularInstruction::CreateSharedMut,
                RegularInstruction::Null,
            )
        );
    }

    #[test]
    fn is_operator() {
        // TODO #151: compare refs
        let datex_script = "1u8 is 2u8".to_string();
        let result = compile_and_log(&datex_script);
        assert_eq!(
            result,
            vec![
                InstructionCode::IS.into(),
                InstructionCode::UINT_8.into(),
                1,
                InstructionCode::UINT_8.into(),
                2
            ]
        );

        let datex_script =
            "const a = shared mut 42u8; const b = 'mut 69u8; a is b"
                .to_string(); // a is b
        let result = compile_and_log(&datex_script);
        assert_eq!(
            result,
            vec![
                InstructionCode::SHORT_STATEMENTS.into(),
                3,
                0, // not terminated
                InstructionCode::PUSH_TO_STACK.into(),
                InstructionCode::CREATE_SHARED_MUT.into(),
                InstructionCode::UINT_8.into(),
                42,
                // val b = 69;
                InstructionCode::PUSH_TO_STACK.into(),
                InstructionCode::DERIVE_SHARED_REF_MUT.into(),
                InstructionCode::UINT_8.into(),
                69,
                // a is b
                InstructionCode::IS.into(),
                InstructionCode::BORROW_STACK_VALUE.into(),
                0,
                0,
                0,
                0, // slot address for a
                InstructionCode::BORROW_STACK_VALUE.into(),
                1,
                0,
                0,
                0, // slot address for b
            ]
        );
    }

    #[test]
    fn equality_operator() {
        let lhs: u8 = 1;
        let rhs: u8 = 2;
        let datex_script = format!("{lhs}u8 == {rhs}u8"); // 1 == 2
        let result = compile_and_log(&datex_script);
        assert_eq!(
            result,
            vec![
                InstructionCode::STRUCTURAL_EQUAL.into(),
                InstructionCode::UINT_8.into(),
                lhs,
                InstructionCode::UINT_8.into(),
                rhs,
            ]
        );

        let datex_script = format!("{lhs}u8 === {rhs}u8"); // 1 === 2
        let result = compile_and_log(&datex_script);
        assert_eq!(
            result,
            vec![
                InstructionCode::EQUAL.into(),
                InstructionCode::UINT_8.into(),
                lhs,
                InstructionCode::UINT_8.into(),
                rhs,
            ]
        );

        let datex_script = format!("{lhs}u8 != {rhs}u8"); // 1 != 2
        let result = compile_and_log(&datex_script);
        assert_eq!(
            result,
            vec![
                InstructionCode::NOT_STRUCTURAL_EQUAL.into(),
                InstructionCode::UINT_8.into(),
                lhs,
                InstructionCode::UINT_8.into(),
                rhs,
            ]
        );
        let datex_script = format!("{lhs}u8 !== {rhs}u8"); // 1 !== 2
        let result = compile_and_log(&datex_script);
        assert_eq!(
            result,
            vec![
                InstructionCode::NOT_EQUAL.into(),
                InstructionCode::UINT_8.into(),
                lhs,
                InstructionCode::UINT_8.into(),
                rhs,
            ]
        );
    }

    #[test]
    fn simple_addition() {
        let lhs: u8 = 1;
        let rhs: u8 = 2;
        let datex_script = format!("{lhs}u8 + {rhs}u8"); // 1 + 2
        let result = compile_and_log(&datex_script);
        assert_eq!(
            result,
            vec![
                InstructionCode::ADD.into(),
                InstructionCode::UINT_8.into(),
                lhs,
                InstructionCode::UINT_8.into(),
                rhs
            ]
        );

        let datex_script = format!("{lhs}u8 + {rhs}u8;"); // 1 + 2;
        let result = compile_and_log(&datex_script);
        assert_eq!(
            result,
            vec![
                InstructionCode::SHORT_STATEMENTS.into(),
                1,
                1, // terminated
                InstructionCode::ADD.into(),
                InstructionCode::UINT_8.into(),
                lhs,
                InstructionCode::UINT_8.into(),
                rhs,
            ]
        );
    }

    #[test]
    fn multi_addition() {
        let op1: u8 = 1;
        let op2: u8 = 2;
        let op3: u8 = 3;
        let op4: u8 = 4;

        let datex_script = format!("{op1}u8 + {op2}u8 + {op3}u8 + {op4}u8"); // 1 + 2 + 3 + 4
        let result = compile_and_log(&datex_script);
        assert_eq!(
            result,
            vec![
                InstructionCode::ADD.into(),
                InstructionCode::ADD.into(),
                InstructionCode::ADD.into(),
                InstructionCode::UINT_8.into(),
                op1,
                InstructionCode::UINT_8.into(),
                op2,
                InstructionCode::UINT_8.into(),
                op3,
                InstructionCode::UINT_8.into(),
                op4,
            ]
        );
    }

    #[test]
    fn mixed_calculation() {
        let op1: u8 = 1;
        let op2: u8 = 2;
        let op3: u8 = 3;
        let op4: u8 = 4;

        let datex_script = format!("{op1}u8 * {op2}u8 + {op3}u8 * {op4}u8"); // 1 + 2 + 3 + 4
        let result = compile_and_log(&datex_script);
        assert_eq!(
            result,
            vec![
                InstructionCode::ADD.into(),
                InstructionCode::MULTIPLY.into(),
                InstructionCode::UINT_8.into(),
                op1,
                InstructionCode::UINT_8.into(),
                op2,
                InstructionCode::MULTIPLY.into(),
                InstructionCode::UINT_8.into(),
                op3,
                InstructionCode::UINT_8.into(),
                op4,
            ]
        );
    }

    #[test]
    fn complex_addition() {
        let a: u8 = 1;
        let b: u8 = 2;
        let c: u8 = 3;
        let datex_script = format!("{a}u8 + ({b}u8 + {c}u8)"); // 1 + (2 + 3)
        let result = compile_and_log(&datex_script);

        assert_eq!(
            result,
            vec![
                InstructionCode::ADD.into(),
                InstructionCode::UINT_8.into(),
                a,
                InstructionCode::ADD.into(),
                InstructionCode::UINT_8.into(),
                b,
                InstructionCode::UINT_8.into(),
                c,
            ]
        );
    }

    #[test]
    fn complex_addition_and_subtraction() {
        let a: u8 = 1;
        let b: u8 = 2;
        let c: u8 = 3;
        let datex_script = format!("{a}u8 + ({b}u8 - {c}u8)"); // 1 + (2 - 3)
        let result = compile_and_log(&datex_script);
        assert_eq!(
            result,
            vec![
                InstructionCode::ADD.into(),
                InstructionCode::UINT_8.into(),
                a,
                InstructionCode::SUBTRACT.into(),
                InstructionCode::UINT_8.into(),
                b,
                InstructionCode::UINT_8.into(),
                c,
            ]
        );
    }

    #[test]
    fn integer_u8() {
        let val = 42;
        let datex_script = format!("{val}u8"); // 42
        let result = compile_and_log(&datex_script);
        assert_eq!(result, vec![InstructionCode::UINT_8.into(), val,]);
    }

    #[test]
    fn range_i64() {
        let start = 128i64;
        let end = 256i64;
        let datex_script = format!("{start}..{end}");
        let result = compile_and_log(&datex_script);

        assert_instructions_equal!(
            &result,
            (
                Instruction::Regular(RegularInstruction::Range),
                Instruction::Regular(RegularInstruction::Integer(
                    Integer::new(start)
                )),
                Instruction::Regular(RegularInstruction::Integer(
                    Integer::new(end)
                ))
            )
        )
    }

    // Test for decimal
    #[test]
    fn decimal() {
        let datex_script = "42.0";
        let result = compile_and_log(datex_script);
        let bytes = 42_i16.to_le_bytes();

        let mut expected: Vec<u8> =
            vec![InstructionCode::DECIMAL_AS_INT_16.into()];
        expected.extend(bytes);

        assert_eq!(result, expected);
    }

    /// Test for test that is less than 256 characters
    #[test]
    fn short_text() {
        let val = "unyt";
        let datex_script = format!("\"{val}\""); // "unyt"
        let result = compile_and_log(&datex_script);
        let mut expected: Vec<u8> =
            vec![InstructionCode::SHORT_TEXT.into(), val.len() as u8];
        expected.extend(val.bytes());
        assert_eq!(result, expected);
    }

    // Test empty list
    #[test]
    fn empty_list() {
        // TODO #437: support list constructor (apply on type)
        let datex_script = "[]";
        // const x = mut 42;
        let result = compile_and_log(datex_script);
        let expected: Vec<u8> = vec![
            InstructionCode::SHORT_LIST.into(),
            0, // length
        ];
        assert_eq!(result, expected);
    }

    // Test list with single element
    #[test]
    fn single_element_list() {
        // TODO #438: support list constructor (apply on type)
        let datex_script = "[42u8]";
        let result = compile_and_log(datex_script);
        assert_eq!(
            result,
            vec![
                InstructionCode::SHORT_LIST.into(),
                1, // length
                InstructionCode::UINT_8.into(),
                42,
            ]
        );
    }

    // Test list with multiple elements
    #[test]
    fn multi_element_list() {
        let datex_script = "[1u8, 2u8, 3u8]";
        let result = compile_and_log(datex_script);
        assert_eq!(
            result,
            vec![
                InstructionCode::SHORT_LIST.into(),
                3, // length
                InstructionCode::UINT_8.into(),
                1,
                InstructionCode::UINT_8.into(),
                2,
                InstructionCode::UINT_8.into(),
                3,
            ]
        );

        // trailing comma
        let datex_script = "[1u8, 2u8, 3u8,]";
        let result = compile_and_log(datex_script);
        assert_eq!(
            result,
            vec![
                InstructionCode::SHORT_LIST.into(),
                3, // length
                InstructionCode::UINT_8.into(),
                1,
                InstructionCode::UINT_8.into(),
                2,
                InstructionCode::UINT_8.into(),
                3,
            ]
        );
    }

    // Test list with expressions inside
    #[test]
    fn list_with_expressions() {
        let datex_script = "[1u8 + 2u8, 3u8 * 4u8]";
        let result = compile_and_log(datex_script);
        assert_eq!(
            result,
            vec![
                InstructionCode::SHORT_LIST.into(),
                2, // length
                InstructionCode::ADD.into(),
                InstructionCode::UINT_8.into(),
                1,
                InstructionCode::UINT_8.into(),
                2,
                InstructionCode::MULTIPLY.into(),
                InstructionCode::UINT_8.into(),
                3,
                InstructionCode::UINT_8.into(),
                4,
            ]
        );
    }

    // Nested lists
    #[test]
    fn nested_lists() {
        let datex_script = "[1u8, [2u8, 3u8], 4u8]";
        let result = compile_and_log(datex_script);
        assert_eq!(
            result,
            vec![
                InstructionCode::SHORT_LIST.into(),
                3, // length
                InstructionCode::UINT_8.into(),
                1,
                InstructionCode::SHORT_LIST.into(),
                2, // length
                InstructionCode::UINT_8.into(),
                2,
                InstructionCode::UINT_8.into(),
                3,
                InstructionCode::UINT_8.into(),
                4,
            ]
        );
    }

    // map with text key
    #[test]
    fn map_with_text_key() {
        let datex_script = "{\"key\": 42u8}";
        let result = compile_and_log(datex_script);
        let expected = vec![
            InstructionCode::SHORT_MAP.into(),
            1, // length
            InstructionCode::KEY_VALUE_SHORT_TEXT.into(),
            3, // length of "key"
            b'k',
            b'e',
            b'y',
            InstructionCode::UINT_8.into(),
            42,
        ];
        assert_eq!(result, expected);
    }

    // map with integer key
    #[test]
    fn map_integer_key() {
        let datex_script = "{(10u8): 42u8}";
        let result = compile_and_log(datex_script);
        let expected = vec![
            InstructionCode::SHORT_MAP.into(),
            1, // length
            InstructionCode::KEY_VALUE_DYNAMIC.into(),
            InstructionCode::UINT_8.into(),
            10,
            InstructionCode::UINT_8.into(),
            42,
        ];
        assert_eq!(result, expected);
    }

    // map with long text key (>255 bytes)
    #[test]
    fn map_with_long_text_key() {
        let long_key = "a".repeat(300);
        let datex_script = format!("{{\"{long_key}\": 42u8}}");
        let result = compile_and_log(&datex_script);
        let mut expected: Vec<u8> = vec![
            InstructionCode::SHORT_MAP.into(),
            1, // length
            InstructionCode::KEY_VALUE_DYNAMIC.into(),
            InstructionCode::TEXT.into(),
        ];
        expected.extend((long_key.len() as u32).to_le_bytes());
        expected.extend(long_key.as_bytes());
        expected.extend(vec![InstructionCode::UINT_8.into(), 42]);
        assert_eq!(result, expected);
    }

    // map with dynamic key (expression)
    #[test]
    fn map_with_dynamic_key() {
        let datex_script = "{(1u8 + 2u8): 42u8}";
        let result = compile_and_log(datex_script);
        let expected = [
            InstructionCode::SHORT_MAP.into(),
            1, // length
            InstructionCode::KEY_VALUE_DYNAMIC.into(),
            InstructionCode::ADD.into(),
            InstructionCode::UINT_8.into(),
            1,
            InstructionCode::UINT_8.into(),
            2,
            InstructionCode::UINT_8.into(),
            42,
        ];
        assert_eq!(result, expected);
    }

    // map with multiple keys (text, integer, expression)
    #[test]
    fn map_with_multiple_keys() {
        let datex_script = "{key: 42u8, (4u8): 43u8, (1u8 + 2u8): 44u8}";
        let result = compile_and_log(datex_script);
        let expected = vec![
            InstructionCode::SHORT_MAP.into(),
            3, // length
            InstructionCode::KEY_VALUE_SHORT_TEXT.into(),
            3, // length of "key"
            b'k',
            b'e',
            b'y',
            InstructionCode::UINT_8.into(),
            42,
            InstructionCode::KEY_VALUE_DYNAMIC.into(),
            InstructionCode::UINT_8.into(),
            4,
            InstructionCode::UINT_8.into(),
            43,
            InstructionCode::KEY_VALUE_DYNAMIC.into(),
            InstructionCode::ADD.into(),
            InstructionCode::UINT_8.into(),
            1,
            InstructionCode::UINT_8.into(),
            2,
            InstructionCode::UINT_8.into(),
            44,
        ];
        assert_eq!(result, expected);
    }

    // empty map
    #[test]
    fn empty_map() {
        let datex_script = "{}";
        let result = compile_and_log(datex_script);
        let expected: Vec<u8> = vec![
            InstructionCode::SHORT_MAP.into(),
            0, // length
        ];
        assert_eq!(result, expected);
    }

    #[test]
    fn empty_tag() {
        let datex_script = "#Example";
        let result = compile_and_log(datex_script);
        assert_instructions_equal!(
            &result,
            (RegularInstruction::TaggedValue(TaggedValue {
                tag: ShortTextData("Example".to_string()),
                is_empty: true,
            }))
        )
    }

    #[test]
    fn tag_with_map() {
        let datex_script = "#Example {a: true}";
        let result = compile_and_log(datex_script);
        assert_instructions_equal!(
            &result,
            (
                RegularInstruction::TaggedValue(TaggedValue {
                    tag: ShortTextData("Example".to_string()),
                    is_empty: false,
                }),
                RegularInstruction::ShortMap(ShortMapData { element_count: 1 }),
                RegularInstruction::KeyValueShortText(ShortTextData(
                    "a".to_string()
                )),
                RegularInstruction::True,
            )
        )
    }

    #[test]
    fn tag_with_single_value() {
        let datex_script = "#Example (\"test\")";
        let result = compile_and_log(datex_script);
        assert_instructions_equal!(
            &result,
            (
                RegularInstruction::TaggedValue(TaggedValue {
                    tag: ShortTextData("Example".to_string()),
                    is_empty: false,
                }),
                RegularInstruction::ShortText(ShortTextData(
                    "test".to_string()
                )),
            )
        )
    }

    #[test]
    fn tag_with_null_value() {
        let datex_script = "#Example (null)";
        let result = compile_and_log(datex_script);
        assert_instructions_equal!(
            &result,
            (
                RegularInstruction::TaggedValue(TaggedValue {
                    tag: ShortTextData("Example".to_string()),
                    is_empty: false,
                }),
                RegularInstruction::Null,
            )
        )
    }

    #[test]
    fn allocate_variable() {
        let script = "const a = 42u8";
        let result = compile_and_log(script);
        assert_eq!(
            result,
            vec![
                InstructionCode::PUSH_TO_STACK.into(),
                InstructionCode::UINT_8.into(),
                42,
            ]
        );
    }

    #[test]
    fn allocate_and_access_variable() {
        let script = "const a = 42u8; a + 1u8";
        let result = compile_and_log(script);
        assert_eq!(
            result,
            vec![
                InstructionCode::SHORT_STATEMENTS.into(),
                2,
                0, // not terminated
                InstructionCode::PUSH_TO_STACK.into(),
                InstructionCode::UINT_8.into(),
                42,
                InstructionCode::ADD.into(),
                InstructionCode::TAKE_STACK_VALUE.into(),
                // slot index as u32
                0,
                0,
                0,
                0,
                InstructionCode::UINT_8.into(),
                1,
            ]
        );
    }

    #[test]
    fn allocate_scoped_variables() {
        let script = "const a = 42u8; (const a = 43u8; a); a";
        let result = compile_and_log(script);
        assert_eq!(
            result,
            vec![
                InstructionCode::SHORT_STATEMENTS.into(),
                3,
                0, // not terminated
                InstructionCode::PUSH_TO_STACK.into(),
                InstructionCode::UINT_8.into(),
                42,
                InstructionCode::SHORT_STATEMENTS.into(),
                2,
                0, // not terminated
                InstructionCode::PUSH_TO_STACK.into(),
                InstructionCode::UINT_8.into(),
                43,
                InstructionCode::TAKE_STACK_VALUE.into(),
                1,
                0,
                0,
                0,
                InstructionCode::TAKE_STACK_VALUE.into(),
                // slot index as u32
                0,
                0,
                0,
                0,
            ]
        );
    }

    #[test]
    fn allocate_scoped_variables_with_parent_variables() {
        let script =
            "const a = 42u8; const b = 41u8; (const a = 43u8; a; b); a";
        let result = compile_and_log(script);
        assert_eq!(
            result,
            vec![
                InstructionCode::SHORT_STATEMENTS.into(),
                4,
                0, // not terminated
                InstructionCode::PUSH_TO_STACK.into(),
                InstructionCode::UINT_8.into(),
                42,
                InstructionCode::PUSH_TO_STACK.into(),
                InstructionCode::UINT_8.into(),
                41,
                InstructionCode::SHORT_STATEMENTS.into(),
                3,
                0, // not terminated
                InstructionCode::PUSH_TO_STACK.into(),
                InstructionCode::UINT_8.into(),
                43,
                InstructionCode::TAKE_STACK_VALUE.into(),
                2,
                0,
                0,
                0,
                InstructionCode::TAKE_STACK_VALUE.into(),
                1,
                0,
                0,
                0,
                InstructionCode::TAKE_STACK_VALUE.into(),
                // slot index as u32
                0,
                0,
                0,
                0,
            ]
        );
    }

    #[test]
    fn allocate_shared() {
        let script = "const a = shared 42u8";
        let result = compile_and_log(script);
        assert_eq!(
            result,
            vec![
                InstructionCode::PUSH_TO_STACK.into(),
                InstructionCode::CREATE_SHARED.into(),
                InstructionCode::UINT_8.into(),
                42,
            ]
        );
    }

    #[test]
    fn read_shared() {
        let script = "const a = shared 42u8; a";
        let result = compile_and_log(script);
        assert_eq!(
            result,
            vec![
                InstructionCode::SHORT_STATEMENTS.into(),
                2,
                0, // not terminated
                InstructionCode::PUSH_TO_STACK.into(),
                InstructionCode::CREATE_SHARED.into(),
                InstructionCode::UINT_8.into(),
                42,
                InstructionCode::TAKE_STACK_VALUE.into(),
                // slot index as u32
                0,
                0,
                0,
                0,
            ]
        );
    }

    fn compile_template_to_dxb(
        template: &str,
        args: Vec<ValueContainer>,
    ) -> Vec<u8> {
        compile_template(
            template,
            args.into_iter().map(Some).collect(),
            CompileOptions::default(),
            Runtime::stub(),
        )
        .expect("Failed to compile template")
        .dxb_with_shared_values
        .dxb
    }

    #[test]
    fn compile() {
        let result = compile_template_to_dxb(
            "? + ?",
            vec![
                (TypedInteger::from(1u8).into()),
                (TypedInteger::from(2u8).into()),
            ],
        );
        assert_eq!(
            result,
            vec![
                InstructionCode::ADD.into(),
                InstructionCode::UINT_8.into(),
                1,
                InstructionCode::UINT_8.into(),
                2
            ]
        );
    }

    #[test]
    fn compile_macro() {
        let a = TypedInteger::from(1u8);
        let result = compile!(Runtime::stub(), "?", a);
        assert_eq!(
            result.unwrap().dxb_with_shared_values.dxb,
            vec![InstructionCode::UINT_8.into(), 1,]
        );
    }

    #[test]
    fn compile_macro_multi() {
        let result = compile!(
            Runtime::stub(),
            "? + ?",
            TypedInteger::from(1u8),
            TypedInteger::from(2u8)
        );
        assert_eq!(
            result.unwrap().dxb_with_shared_values.dxb,
            vec![
                InstructionCode::ADD.into(),
                InstructionCode::UINT_8.into(),
                1,
                InstructionCode::UINT_8.into(),
                2
            ]
        );
    }

    // TODO #721:
    // #[cfg(feature = "std")]
    // fn get_json_test_string(file_path: &str) -> String {
    //     // read json from test file
    //     let file_path = format!("benches/json/{file_path}");
    //     let file_path = std::path::Path::new(&file_path);
    //     let file =
    //         std::fs::File::open(file_path).expect("Failed to open test.json");
    //     let mut reader = std::io::BufReader::new(file);
    //     let mut json_string = String::new();
    //     reader
    //         .read_to_string(&mut json_string)
    //         .expect("Failed to read test.json");
    //     json_string
    // }
    //
    // #[test]
    // #[cfg(feature = "std")]
    // fn json_to_dxb_large_file() {
    //     let json = get_json_test_string("test3.json");
    //     let _ = compile_script(&json, CompileOptions::default())
    //         .expect("Failed to parse JSON string");
    // }

    #[test]
    fn static_value_detection() {
        // non-static
        let script = "1 + 2";
        let compilation_scope = get_compilation_result(script);
        assert!(compilation_scope.has_non_static_value);

        let script = "1 2";
        let compilation_scope = get_compilation_result(script);
        assert!(compilation_scope.has_non_static_value);

        let script = "1;2";
        let compilation_scope = get_compilation_result(script);
        assert!(compilation_scope.has_non_static_value);

        let script = r#"{("x" + "y"): 1}"#;
        let compilation_scope = get_compilation_result(script);
        assert!(compilation_scope.has_non_static_value);

        // static
        let script = "1";
        let compilation_scope = get_compilation_result(script);
        assert!(!compilation_scope.has_non_static_value);

        let script = "[]";
        let compilation_scope = get_compilation_result(script);
        assert!(!compilation_scope.has_non_static_value);

        let script = "{}";
        let compilation_scope = get_compilation_result(script);
        assert!(!compilation_scope.has_non_static_value);

        let script = "[1,2,3]";
        let compilation_scope = get_compilation_result(script);
        assert!(!compilation_scope.has_non_static_value);

        let script = "{a: 2}";
        let compilation_scope = get_compilation_result(script);
        assert!(!compilation_scope.has_non_static_value);

        // because of unary - 42
        let script = "-42";
        let compilation_scope = get_compilation_result(script);
        assert!(!compilation_scope.has_non_static_value);
    }

    #[test]
    fn compile_auto_static_value_detection() {
        let script = "1u8";
        let res = compile_script_or_return_static_value(
            script,
            CompileOptions::default(),
            Runtime::stub(),
        )
        .unwrap()
        .0;

        assert_matches!(
            res,
            StaticValueOrDXB::StaticValue(val) if val.as_ref().unwrap().try_as::<u8>().unwrap() == &1u8
        );

        let script = "1u8 + 2u8";
        let res = compile_script_or_return_static_value(
            script,
            CompileOptions::default(),
            Runtime::stub(),
        )
        .unwrap()
        .0;

        assert_matches!(
            res,
            StaticValueOrDXB::DXB(DXBWithSharedValues { dxb, ..}) if dxb == vec![
                InstructionCode::ADD.into(),
                InstructionCode::UINT_8.into(),
                1,
                InstructionCode::UINT_8.into(),
                2,
            ]
        );
    }

    #[test]
    fn nested_statements() {
        flexi_logger::init();
        let script = r#"
            var x = 1u8;
            (
                var y = 2u8;
                clone x;
                y;
            );
            var z = 3u8;
            x;
            z;
        "#;
        let res = compile_unwrap(script);
        print_disassembled(&res);
        assert_instructions_equal!(
            &res,
            (RegularInstruction::statements_with_children(
                true,
                instructions!(
                    RegularInstruction::PushToStack,
                    RegularInstruction::uint8(1),
                    RegularInstruction::statements_with_children(
                        true,
                        instructions!(
                            RegularInstruction::PushToStack,
                            RegularInstruction::uint8(2),
                            RegularInstruction::CloneStackValue(StackIndex(0)),
                            RegularInstruction::take_stack_value(StackIndex(1)),
                        )
                    ),
                    RegularInstruction::PushToStack,
                    RegularInstruction::uint8(3),
                    RegularInstruction::take_stack_value(StackIndex(0)),
                    RegularInstruction::take_stack_value(StackIndex(1)),
                )
            ),)
        );
    }

    #[test]
    fn remote_execution() {
        let script = "42u8 :: 43u8";
        let res = compile_unwrap(script);
        assert_eq!(
            res,
            vec![
                InstructionCode::REMOTE_EXECUTION.into(),
                // --- start of block
                // block size (2 bytes)
                2,
                0,
                0,
                0,
                // injected slots (0)
                0,
                0,
                0,
                0,
                // literal value 43
                InstructionCode::UINT_8.into(),
                43,
                // --- end of block
                // caller (literal value 42 for test)
                InstructionCode::UINT_8.into(),
                42,
            ]
        );
    }

    #[test]
    fn remote_execution_expression() {
        let script = "42u8 :: 1u8 + 2u8";
        let res = compile_unwrap(script);
        assert_eq!(
            res,
            vec![
                InstructionCode::REMOTE_EXECUTION.into(),
                // --- start of block
                // block size (5 bytes)
                5,
                0,
                0,
                0,
                // injected slots (0)
                0,
                0,
                0,
                0,
                // expression: 1 + 2
                InstructionCode::ADD.into(),
                InstructionCode::UINT_8.into(),
                1,
                InstructionCode::UINT_8.into(),
                2,
                // --- end of block
                // caller (literal value 42 for test)
                InstructionCode::UINT_8.into(),
                42,
            ]
        );
    }

    #[test]
    fn remote_execution_invalid_reassignment_of_external_variable() {
        flexi_logger::init();
        let script = "var x = 42u8; 1u8 :: (x = 43u8)";
        let result =
            compile_script(script, CompileOptions::default(), Runtime::stub());
        assert!(result.is_err());
        assert_matches!(
            result.err().unwrap().error,
            CompilerError::AssignmentToExternalVariable(name) if name == "x"
        );
    }

    #[test]
    fn remote_execution_injected_const() {
        let script = "const x = 42u8; 1u8 :: x";
        let res = compile_unwrap(script);
        assert_instructions_equal!(
            &res,
            (RegularInstruction::statements_with_children(
                false,
                instructions!(
                    RegularInstruction::PushToStack,
                    RegularInstruction::uint8(42),
                    RegularInstruction::_RemoteExecutionDebugFlat(
                        InstructionBlockDataDebugFlat {
                            length: 5,
                            injected_variable_count: 1,
                            // FIXME should be local
                            injected_values: vec![InjectedValueDeclaration {
                                index: StackIndex(0),
                                ty: InjectedValueType::Shared(
                                    SharedInjectedValueType::Move
                                )
                            }],
                            body: instructions_with_span!(
                                Instruction::Regular(
                                    RegularInstruction::take_stack_value(
                                        StackIndex(0)
                                    )
                                )
                            )
                        }
                    ),
                    RegularInstruction::uint8(1),
                )
            ),)
        );
    }

    #[test]
    fn remote_execution_injected_shared_move() {
        // var x only refers to a value, not a ref, but since it is transferred to a
        // remote context, its state is synced via a ref (VariableReference model)
        let script = "const x = shared 42u8; 1u8 :: x";
        let res = compile_unwrap(script);
        assert_instructions_equal!(
            &res,
            (RegularInstruction::statements_with_children(
                false,
                instructions!(
                    RegularInstruction::PushToStack,
                    RegularInstruction::CreateShared,
                    RegularInstruction::uint8(42),
                    RegularInstruction::_RemoteExecutionDebugFlat(
                        InstructionBlockDataDebugFlat {
                            length: 5,
                            injected_variable_count: 1,
                            injected_values: vec![InjectedValueDeclaration {
                                index: StackIndex(0),
                                ty: InjectedValueType::Shared(
                                    SharedInjectedValueType::Move
                                )
                            }],
                            body: instructions_with_span!(
                                Instruction::Regular(
                                    RegularInstruction::take_stack_value(
                                        StackIndex(0)
                                    )
                                )
                            ),
                        }
                    ),
                    RegularInstruction::uint8(1),
                )
            ),)
        )
    }

    #[test]
    fn remote_execution_injected_shared_ref() {
        let script = "const x = shared 42u8; 1u8 :: 'x";
        let res = compile_unwrap(script);
        assert_instructions_equal!(
            &res,
            (RegularInstruction::statements_with_children(
                false,
                instructions!(
                    RegularInstruction::PushToStack,
                    RegularInstruction::CreateShared,
                    RegularInstruction::uint8(42),
                    RegularInstruction::_RemoteExecutionDebugFlat(
                        InstructionBlockDataDebugFlat {
                            length: 5,
                            injected_variable_count: 1,
                            injected_values: vec![InjectedValueDeclaration {
                                index: StackIndex(0),
                                ty: InjectedValueType::Shared(
                                    SharedInjectedValueType::Ref
                                )
                            }],
                            body: instructions_with_span!(Instruction::Regular(
                                RegularInstruction::get_stack_value_shared_ref(
                                    StackIndex(0)
                                )
                            )),
                        }
                    ),
                    RegularInstruction::uint8(1),
                )
            ),)
        )
    }

    #[test]
    fn remote_execution_injected_shared_ref_and_move() {
        let script = "const x = shared 42u8; 1u8 :: ('x; x)";
        let res = compile_unwrap(script);
        assert_instructions_equal!(
            &res,
            (RegularInstruction::statements_with_children(
                false,
                instructions!(
                    RegularInstruction::PushToStack,
                    RegularInstruction::CreateShared,
                    RegularInstruction::uint8(42),
                    RegularInstruction::_RemoteExecutionDebugTree(
                        InstructionBlockDataDebugTree {
                            length: 13,
                            injected_variable_count: 1,
                            injected_values: vec![InjectedValueDeclaration {
                                index: StackIndex(0),
                                ty: InjectedValueType::Shared(
                                    SharedInjectedValueType::Move
                                )
                            }],
                            body: RegularInstruction::statements_with_children(
                                false,
                                instructions!(
                                    RegularInstruction::get_stack_value_shared_ref(
                                        StackIndex(0)
                                    ),
                                    RegularInstruction::take_stack_value(
                                        StackIndex(0)
                                    )
                                )
                            ).into(),
                        }
                    ),
                    RegularInstruction::uint8(1),
                )
            ),)
        )
    }

    #[test]
    fn remote_execution_injected_consts() {
        let script = "const x = 42u8; const y = 69u8; 1u8 :: x + y";
        let res = compile_unwrap(script);
        assert_instructions_equal!(
            &res,
            (RegularInstruction::statements_with_children(
                false,
                instructions!(
                    RegularInstruction::PushToStack,
                    RegularInstruction::uint8(42),
                    RegularInstruction::PushToStack,
                    RegularInstruction::uint8(69),
                    RegularInstruction::_RemoteExecutionDebugFlat(
                        InstructionBlockDataDebugFlat {
                            length: 11,
                            injected_variable_count: 2,
                            injected_values: vec![
                                // FIXME should be local
                                InjectedValueDeclaration {
                                    index: StackIndex(0),
                                    ty: InjectedValueType::Shared(
                                        SharedInjectedValueType::Move
                                    )
                                },
                                InjectedValueDeclaration {
                                    index: StackIndex(1),
                                    ty: InjectedValueType::Shared(
                                        SharedInjectedValueType::Move
                                    )
                                },
                            ],
                            body: instructions_with_span!(
                                Instruction::Regular(RegularInstruction::add()),
                                Instruction::Regular(
                                    RegularInstruction::take_stack_value(
                                        StackIndex(0)
                                    )
                                ),
                                Instruction::Regular(
                                    RegularInstruction::take_stack_value(
                                        StackIndex(1)
                                    )
                                ),
                            ),
                        }
                    ),
                    RegularInstruction::uint8(1),
                )
            ),)
        );
    }

    #[test]
    fn remote_execution_shadow_const() {
        let script =
            "const x = 42u8; const y = 69u8; 1u8 :: (const x = 5u8; x + y)";
        let res = compile_unwrap(script);
        assert_instructions_equal!(
            &res,
            (RegularInstruction::statements_with_children(
                false,
                instructions!(
                    RegularInstruction::PushToStack,
                    RegularInstruction::uint8(42),
                    RegularInstruction::PushToStack,
                    RegularInstruction::uint8(69),
                    RegularInstruction::_RemoteExecutionDebugTree(
                        InstructionBlockDataDebugTree {
                            length: 17,
                            injected_variable_count: 1,
                            injected_values: vec![
                                // FIXME should be local
                                InjectedValueDeclaration {
                                    index: StackIndex(1),
                                    ty: InjectedValueType::Shared(
                                        SharedInjectedValueType::Move
                                    )
                                },
                            ],
                            body: RegularInstruction::statements_with_children(
                                false,
                                instructions!(
                                    RegularInstruction::PushToStack,
                                    RegularInstruction::uint8(5),
                                    RegularInstruction::Add,
                                    RegularInstruction::take_stack_value(
                                        StackIndex(1)
                                    ),
                                    RegularInstruction::take_stack_value(
                                        StackIndex(0)
                                    ),
                                )
                            )
                            .into(),
                        }
                    ),
                    RegularInstruction::uint8(1),
                )
            ),)
        );
    }

    #[test]
    fn remote_execution_nested() {
        let script = "const x = 42u8; (1u8 :: (2u8 :: x))";
        let res = compile_unwrap(script);

        assert_eq!(
            res,
            vec![
                InstructionCode::SHORT_STATEMENTS.into(),
                2,
                0, // not terminated
                InstructionCode::PUSH_TO_STACK.into(),
                InstructionCode::UINT_8.into(),
                42,
                InstructionCode::REMOTE_EXECUTION.into(),
                // --- start of block 1
                // block size (20 bytes)
                21,
                0,
                0,
                0,
                // injected slots (1)
                1,
                0,
                0,
                0,
                // slot 0
                0,
                0,
                0,
                0,
                // FIXME
                InjectedValueType::Shared(SharedInjectedValueType::Move).into(),
                // nested remote execution
                InstructionCode::REMOTE_EXECUTION.into(),
                // --- start of block 2
                // block size (5 bytes)
                5,
                0,
                0,
                0,
                // injected slots (1)
                1,
                0,
                0,
                0,
                // slot 0
                0,
                0,
                0,
                0,
                // FIXME
                InjectedValueType::Shared(SharedInjectedValueType::Move).into(),
                InstructionCode::TAKE_STACK_VALUE.into(),
                // slot index as u32
                0,
                0,
                0,
                0,
                // --- end of block 2
                // caller (literal value 2 for test)
                InstructionCode::UINT_8.into(),
                2,
                // -- end of block 1
                // caller (literal value 1 for test)
                InstructionCode::UINT_8.into(),
                1,
            ]
        );
    }

    #[test]
    fn remote_execution_nested2() {
        let script = "const x = 42u8; const y = 43u8; (1u8 :: (y :: x))";
        let res = compile_unwrap(script);

        assert_eq!(
            res,
            vec![
                InstructionCode::SHORT_STATEMENTS.into(),
                3,
                0, // not terminated
                InstructionCode::PUSH_TO_STACK.into(),
                InstructionCode::UINT_8.into(),
                42,
                InstructionCode::PUSH_TO_STACK.into(),
                InstructionCode::UINT_8.into(),
                43,
                InstructionCode::REMOTE_EXECUTION.into(),
                // --- start of block 1
                // block size (24 bytes)
                24,
                0,
                0,
                0,
                // injected slots (2)
                2,
                0,
                0,
                0,
                // slot 1
                0,
                0,
                0,
                0,
                // FIXME
                InjectedValueType::Shared(SharedInjectedValueType::Move).into(),
                // slot 0
                1,
                0,
                0,
                0,
                // FIXME
                InjectedValueType::Shared(SharedInjectedValueType::Move).into(),
                // nested remote execution
                InstructionCode::REMOTE_EXECUTION.into(),
                // --- start of block 2
                // block size (5 bytes)
                5,
                0,
                0,
                0,
                // injected slots (1)
                1,
                0,
                0,
                0,
                // slot 0
                0,
                0,
                0,
                0,
                // FIXME
                InjectedValueType::Shared(SharedInjectedValueType::Move).into(),
                InstructionCode::TAKE_STACK_VALUE.into(),
                // slot index as u32
                0,
                0,
                0,
                0,
                // --- end of block 2
                // caller (literal value 2 for test)
                InstructionCode::TAKE_STACK_VALUE.into(),
                1,
                0,
                0,
                0,
                // --- end of block 1
                // caller (literal value 1 for test)
                InstructionCode::UINT_8.into(),
                1,
            ]
        );
    }

    #[test]
    fn assignment_to_const() {
        let script = "const a = 42; a = 43";
        let result =
            compile_script(script, CompileOptions::default(), Runtime::stub())
                .map_err(|e| e.error);
        assert_matches!(result, Err(CompilerError::AssignmentToConst { .. }));
    }

    #[test]
    fn assignment_to_const_mut() {
        let script = "const a = &mut 42; a = 43";
        let result =
            compile_script(script, CompileOptions::default(), Runtime::stub())
                .map_err(|e| e.error);
        assert_matches!(result, Err(CompilerError::AssignmentToConst { .. }));
    }

    #[test]
    fn internal_assignment_to_const_mut() {
        let script = "const a = &mut 42; *a = 43";
        let result =
            compile_script(script, CompileOptions::default(), Runtime::stub());
        assert_matches!(result, Ok(_));
    }

    #[test]
    fn addition_to_const_mut_ref() {
        let script = "const a = &mut 42; *a += 1;";
        let result =
            compile_script(script, CompileOptions::default(), Runtime::stub());
        assert_matches!(result, Ok(_));
    }

    #[test]
    fn addition_to_const_variable() {
        let script = "const a = 42; a += 1";
        let result =
            compile_script(script, CompileOptions::default(), Runtime::stub())
                .map_err(|e| e.error);
        assert_matches!(result, Err(CompilerError::AssignmentToConst { .. }));
    }

    #[test]
    fn root_property_endpoint() {
        let script = "$.endpoint";
        let res = compile_unwrap(script);
        assert_instructions_equal!(
            &res,
            (RegularInstruction::GetRootProperty(RootProperty::ENDPOINT))
        );
    }

    #[test]
    fn root_property_caller() {
        let script = "$.caller";
        let res = compile_unwrap(script);
        assert_instructions_equal!(
            &res,
            (RegularInstruction::GetRootProperty(RootProperty::CALLER))
        );
    }

    #[test]
    fn callable_declaration() {
        let script = "function add(a: integer, b: integer) -> integer (a + b)";
        let res = compile_unwrap(script);
        assert_instructions_equal!(
            &res,
            (RegularInstruction::_CallableDeclarationDebugTree(
                CallableDeclarationDataDebugTree {
                    signature: CallableSignatureData {
                        has_rest_parameter: false,
                        requires_async: false,
                        name: ShortTextData("add".to_string()),
                        kind: CallableKind::Function,
                        parameter_names: vec![
                            ShortTextData("a".to_string()),
                            ShortTextData("b".to_string())
                        ],
                        has_return_type: true,
                        has_yeet_type: false,
                        parameter_count: 2,
                        rest_parameter_name: None,
                    },
                    body: InstructionBlockDataDebugTree {
                        length: 11,
                        injected_variable_count: 0,
                        injected_values: vec![],
                        body: RegularInstruction::Add
                            .with_children(instructions!(
                                RegularInstruction::TakeStackValue(StackIndex(
                                    0
                                )),
                                RegularInstruction::TakeStackValue(StackIndex(
                                    1
                                )),
                            ))
                            .into()
                    }
                }
            )
            .with_children(instructions!(
                // parameter types
                TypeInstruction::CoreType(CoreLibBaseTypeId::Integer.into()),
                TypeInstruction::CoreType(CoreLibBaseTypeId::Integer.into()),
                // return type
                TypeInstruction::CoreType(CoreLibBaseTypeId::Integer.into())
            )))
        )
    }

    #[test]
    fn callable_declaration_with_injected_value() {
        let script =
            "const x = 42; function add(a: integer) -> integer (x + a)";
        let res = compile_unwrap(script);
        assert_instructions_equal!(
            &res,
            (RegularInstruction::statements_with_children(
                false,
                instructions!(
                    RegularInstruction::PushToStack.with_children(
                        instructions!(RegularInstruction::Integer(
                            Integer::from(42)
                        ))
                    ),
                    RegularInstruction::_CallableDeclarationDebugTree(
                        CallableDeclarationDataDebugTree {
                            signature: CallableSignatureData {
                                has_rest_parameter: false,
                                name: ShortTextData("add".to_string()),
                                kind: CallableKind::Function,
                                requires_async: false,
                                parameter_names: vec![ShortTextData(
                                    "a".to_string()
                                ),],
                                has_return_type: true,
                                has_yeet_type: false,
                                parameter_count: 1,
                                rest_parameter_name: None,
                            },
                            body: InstructionBlockDataDebugTree {
                                length: 11,
                                injected_variable_count: 1,
                                injected_values: vec![
                                    InjectedValueDeclaration {
                                        index: StackIndex(0),
                                        ty: InjectedValueType::Shared(
                                            SharedInjectedValueType::Move
                                        ),
                                    }
                                ],
                                body: RegularInstruction::Add
                                    .with_children(instructions!(
                                        RegularInstruction::TakeStackValue(
                                            StackIndex(1)
                                        ),
                                        RegularInstruction::TakeStackValue(
                                            StackIndex(0)
                                        ),
                                    ))
                                    .into()
                            }
                        }
                    )
                    .with_children(instructions!(
                        // parameter types
                        TypeInstruction::CoreType(
                            CoreLibBaseTypeId::Integer.into()
                        ),
                        // return type
                        TypeInstruction::CoreType(
                            CoreLibBaseTypeId::Integer.into()
                        )
                    ))
                )
            ))
        );
    }

    // this is not a valid Datex script, just testing the compiler
    #[test]
    fn unbox() {
        let script = "*10u8";
        let res = compile_unwrap(script);
        assert_eq!(
            res,
            vec![
                InstructionCode::UNBOX.into(),
                InstructionCode::UINT_8.into(),
                // integer as u8
                10,
            ]
        );
    }

    #[test]
    fn unbox_slot() {
        let script = "const x = 10u8; *x";
        let res = compile_unwrap(script);
        assert_eq!(
            res,
            vec![
                InstructionCode::SHORT_STATEMENTS.into(),
                2,
                0, // not terminated
                InstructionCode::PUSH_TO_STACK.into(),
                InstructionCode::UINT_8.into(),
                10,
                InstructionCode::UNBOX.into(), // FIXME: should not be added for local values (precompiler)
                InstructionCode::BORROW_STACK_VALUE.into(),
                // slot index as u32
                0,
                0,
                0,
                0,
            ]
        );
    }

    #[test]
    fn type_literal_integer() {
        let script = "type<1>";
        let res = compile_unwrap(script);

        assert_instructions_equal!(
            &res,
            (
                Instruction::Regular(RegularInstruction::TypeExpression),
                Instruction::Type(TypeInstruction::Literal(
                    LiteralTypeDefinition::Integer(1.into())
                ))
            )
        );
    }

    #[test]
    fn type_core_type_integer() {
        let script = "integer";
        let res = compile_unwrap(script);

        assert_instructions_equal!(
            &res,
            (RegularInstruction::GetCoreLibValue(
                CoreLibId::Type(CoreLibTypeId::Base(
                    CoreLibBaseTypeId::Integer
                ))
                .into()
            ))
        )
    }

    #[test]
    fn compile_continuous_terminated_script() {
        let input = vec!["1u8", "2u8", "3u8;"];
        let expected_output = vec![
            vec![
                InstructionCode::UNBOUNDED_STATEMENTS.into(),
                InstructionCode::UINT_8.into(),
                1,
            ],
            vec![InstructionCode::UINT_8.into(), 2],
            vec![
                InstructionCode::UINT_8.into(),
                3,
                InstructionCode::UNBOUNDED_STATEMENTS_END.into(),
                1, // terminated
            ],
        ];

        assert_unbounded_input_matches_output(input, expected_output);
    }

    #[test]
    fn compile_continuous_unterminated_script() {
        let input = vec!["1u8", "2u8 + 3u8", "3u8"];
        let expected_output = vec![
            vec![
                InstructionCode::UNBOUNDED_STATEMENTS.into(),
                InstructionCode::UINT_8.into(),
                1,
            ],
            vec![
                InstructionCode::ADD.into(),
                InstructionCode::UINT_8.into(),
                2,
                InstructionCode::UINT_8.into(),
                3,
            ],
            vec![
                InstructionCode::UINT_8.into(),
                3,
                InstructionCode::UNBOUNDED_STATEMENTS_END.into(),
                0, // unterminated
            ],
        ];

        assert_unbounded_input_matches_output(input, expected_output);
    }

    #[test]
    fn compile_continuous_complex() {
        let input = vec!["1u8", "integer"];
        let expected_output = vec![
            vec![
                InstructionCode::UNBOUNDED_STATEMENTS.into(),
                InstructionCode::UINT_8.into(),
                1,
            ],
            vec![
                InstructionCode::GET_CORE_LIB_VALUE.into(),
                // pointer id for integer
                3,
                0,
                InstructionCode::UNBOUNDED_STATEMENTS_END.into(),
                0, // unterminated
            ],
        ];

        assert_unbounded_input_matches_output(input, expected_output);
    }

    #[test]
    fn get_property_text() {
        let datex_script = r#""test".example"#;
        let result = compile_and_log(datex_script);
        let expected = vec![
            InstructionCode::GET_ENTRY_TEXT.into(),
            7, // length of "example"
            b'e',
            b'x',
            b'a',
            b'm',
            b'p',
            b'l',
            b'e',
            // base value
            InstructionCode::SHORT_TEXT.into(),
            4, // length of "test"
            b't',
            b'e',
            b's',
            b't',
        ];
        assert_eq!(result, expected);
    }

    #[test]
    fn get_property_text_quoted() {
        let datex_script = r#""test"."example""#;
        let result = compile_and_log(datex_script);
        let expected = vec![
            InstructionCode::GET_ENTRY_TEXT.into(),
            7, // length of "example"
            b'e',
            b'x',
            b'a',
            b'm',
            b'p',
            b'l',
            b'e',
            // base value
            InstructionCode::SHORT_TEXT.into(),
            4, // length of "test"
            b't',
            b'e',
            b's',
            b't',
        ];
        assert_eq!(result, expected);
    }

    #[test]
    fn get_property_index() {
        let datex_script = r#""test".42"#;
        let result = compile_and_log(datex_script);
        let expected = vec![
            InstructionCode::GET_ENTRY_INDEX.into(),
            // u32 index 42
            42,
            0,
            0,
            0,
            // base value
            InstructionCode::SHORT_TEXT.into(),
            4, // length of "test"
            b't',
            b'e',
            b's',
            b't',
        ];
        assert_eq!(result, expected);
    }

    #[test]
    fn get_property_dynamic() {
        let datex_script = r#""test".(1u8 + 2u8)"#;
        let result = compile_and_log(datex_script);
        let expected = vec![
            InstructionCode::GET_ENTRY_DYNAMIC.into(),
            // property expression: 1 + 2
            InstructionCode::ADD.into(),
            InstructionCode::UINT_8.into(),
            1,
            InstructionCode::UINT_8.into(),
            2,
            // base value
            InstructionCode::SHORT_TEXT.into(),
            4, // length of "test"
            b't',
            b'e',
            b's',
            b't',
        ];
        assert_eq!(result, expected);
    }

    #[test]
    fn set_property_text() {
        let datex_script = r#""test".example = 42u8"#;
        let result = compile_and_log(datex_script);
        let expected = vec![
            InstructionCode::SET_ENTRY_TEXT.into(),
            7, // length of "example"
            b'e',
            b'x',
            b'a',
            b'm',
            b'p',
            b'l',
            b'e',
            // value to set
            InstructionCode::UINT_8.into(),
            42,
            // base value
            InstructionCode::SHORT_TEXT.into(),
            4, // length of "test"
            b't',
            b'e',
            b's',
            b't',
        ];
        assert_eq!(result, expected);
    }

    #[test]
    fn set_property_index() {
        let datex_script = r#""test".42 = 43u8"#;
        let result = compile_and_log(datex_script);
        let expected = vec![
            InstructionCode::SET_ENTRY_INDEX.into(),
            // u32 index 42
            42,
            0,
            0,
            0,
            // value to set
            InstructionCode::UINT_8.into(),
            43,
            // base value
            InstructionCode::SHORT_TEXT.into(),
            4, // length of "test"
            b't',
            b'e',
            b's',
            b't',
        ];
        assert_eq!(result, expected);
    }

    #[test]
    fn set_property_dynamic() {
        let datex_script = r#""test".(1u8 + 2u8) = 43u8"#;
        let result = compile_and_log(datex_script);
        let expected = vec![
            InstructionCode::SET_ENTRY_DYNAMIC.into(),
            // property expression: 1 + 2
            InstructionCode::ADD.into(),
            InstructionCode::UINT_8.into(),
            1,
            InstructionCode::UINT_8.into(),
            2,
            // value to set
            InstructionCode::UINT_8.into(),
            43,
            // base value
            InstructionCode::SHORT_TEXT.into(),
            4, // length of "test"
            b't',
            b'e',
            b's',
            b't',
        ];
        assert_eq!(result, expected);
    }

    #[test]
    fn apply_callable_no_arguments() {
        let script = "function add() (null) ()";
        let res = compile_unwrap(script);
        assert_instructions_equal!(
            &res,
            (RegularInstruction::apply(0).with_children(instructions!(
                RegularInstruction::_CallableDeclarationDebugTree(
                    CallableDeclarationDataDebugTree {
                        signature: CallableSignatureData {
                            has_rest_parameter: false,
                            name: ShortTextData("add".to_string()),
                            kind: CallableKind::Function,
                            requires_async: false,
                            parameter_names: vec![],
                            has_return_type: false,
                            has_yeet_type: false,
                            parameter_count: 0,
                            rest_parameter_name: None,
                        },
                        body: InstructionBlockDataDebugTree {
                            length: 1,
                            injected_variable_count: 0,
                            injected_values: vec![],
                            body: Instruction::Regular(
                                RegularInstruction::Null
                            )
                            .into()
                        }
                    }
                )
            )))
        )
    }

    #[test]
    fn apply_no_arguments() {
        let datex_script = r#""test"()"#;
        let result = compile_and_log(datex_script);
        assert_instructions_equal!(
            &result,
            (RegularInstruction::apply(0).with_children(instructions!(
                RegularInstruction::ShortText(ShortTextData(
                    "test".to_string()
                )),
            )))
        )
    }

    #[test]
    fn apply_one_argument() {
        let datex_script = r#""test" 42u8"#;
        let result = compile_and_log(datex_script);
        assert_instructions_equal!(
            &result,
            (RegularInstruction::apply(1).with_children(instructions!(
                RegularInstruction::uint8(42),
                RegularInstruction::ShortText(ShortTextData(
                    "test".to_string()
                )),
            )))
        );
    }

    #[test]
    fn apply_multiple_arguments() {
        let datex_script = r#""test"(1u8, 2u8, 3u8)"#;
        let result = compile_and_log(datex_script);
        assert_instructions_equal!(
            &result,
            (RegularInstruction::apply(3).with_children(instructions!(
                RegularInstruction::uint8(1),
                RegularInstruction::uint8(2),
                RegularInstruction::uint8(3),
                RegularInstruction::ShortText(ShortTextData(
                    "test".to_string()
                )),
            )))
        );
    }

    #[test]
    fn clone_local_value() {
        let datex_script = "var x = 10u8; var y = clone x; x";
        let result = compile_and_log(datex_script);
        let expected = vec![
            InstructionCode::SHORT_STATEMENTS.into(),
            3,
            0, // not terminated
            InstructionCode::PUSH_TO_STACK.into(),
            InstructionCode::UINT_8.into(),
            10,
            InstructionCode::PUSH_TO_STACK.into(),
            InstructionCode::CLONE_STACK_VALUE.into(),
            // slot index as u32
            0,
            0,
            0,
            0,
            InstructionCode::TAKE_STACK_VALUE.into(),
            // slot index as u32
            0,
            0,
            0,
            0,
        ];
        assert_eq!(result, expected);
    }

    #[test]
    fn variable_shadowing() {
        let datex_script =
            "var x = 42u8; (var x = 43u8;); (var y = 43u8; y); x";
        let result = compile_and_log(datex_script);
        assert_instructions_equal!(
            &result,
            (RegularInstruction::statements_with_children(
                false,
                instructions!(
                    // var x = 42u8
                    RegularInstruction::PushToStack,
                    RegularInstruction::uint8(42),
                    // var x = 43u8;
                    RegularInstruction::statements_with_children(
                        true,
                        instructions!(
                            RegularInstruction::PushToStack,
                            RegularInstruction::uint8(43),
                        )
                    ),
                    // var y = 43u8; y
                    RegularInstruction::statements_with_children(
                        false,
                        instructions!(
                            RegularInstruction::PushToStack,
                            RegularInstruction::uint8(43),
                            RegularInstruction::take_stack_value(StackIndex(1)),
                        )
                    ),
                    // x
                    RegularInstruction::take_stack_value(StackIndex(0))
                )
            ),)
        );
    }

    #[test]
    fn variable_shadowing_2() {
        let datex_script = "var x = 1u8; var y = (var x = 2u8; x); [x, y]";
        let result = compile_and_log(datex_script);
        assert_instructions_equal!(
            &result,
            (RegularInstruction::statements_with_children(
                false,
                instructions!(
                    // var x = 1u8
                    RegularInstruction::PushToStack,
                    RegularInstruction::uint8(1),
                    // var y =
                    RegularInstruction::PushToStack,
                    // (var x = 2u8; x)
                    RegularInstruction::statements_with_children(
                        false,
                        instructions!(
                            RegularInstruction::PushToStack,
                            RegularInstruction::uint8(2),
                            RegularInstruction::take_stack_value(StackIndex(1)),
                        )
                    ),
                    // [x, y]
                    RegularInstruction::list(2),
                    RegularInstruction::take_stack_value(StackIndex(0)),
                    RegularInstruction::take_stack_value(StackIndex(1)),
                )
            ),)
        )
    }

    #[test]
    fn interface_method_calls() {
        let datex_script = "var x = []; x->append(true);";
        let result = compile_and_log(datex_script);
        assert_instructions_equal!(
            &result,
            (RegularInstruction::statements_with_children(
                true,
                instructions!(
                    // var x = []
                    RegularInstruction::PushToStack,
                    RegularInstruction::list(0),
                    // x->append(true)
                    RegularInstruction::AppendEntry.with_children(
                        instructions!(
                            RegularInstruction::True,
                            RegularInstruction::BorrowStackValue(StackIndex(0)),
                        )
                    )
                )
            ),)
        );
    }

    #[test]
    fn conditional_if_else() {
        let script = "
                if (true) (
                    122u32
                )
                else (
                    123u32
                )";
        let result = compile_and_log(script);

        assert_instructions_equal!(
            &result,
            (
                RegularInstruction::unbounded_statements(),
                RegularInstruction::jump_if_false(10),
                RegularInstruction::True,
                RegularInstruction::uint32(122),
                RegularInstruction::jump(5),
                RegularInstruction::uint32(123),
                RegularInstruction::unbounded_statements_end(false),
            )
        );
    }

    #[test]
    fn conditional_with_variable() {
        let script = "const x = 10u8; if (true) (x) else (0u8)";
        let result = compile_and_log(script);

        assert_instructions_equal!(
            &result,
            (RegularInstruction::statements_with_children(
                false,
                instructions!(
                    RegularInstruction::PushToStack,
                    RegularInstruction::uint8(10),
                    instructions!(
                        RegularInstruction::unbounded_statements(),
                        RegularInstruction::jump_if_false(10),
                        RegularInstruction::True,
                        RegularInstruction::take_stack_value(StackIndex(0)),
                        RegularInstruction::jump(2),
                        RegularInstruction::uint8(0),
                        RegularInstruction::unbounded_statements_end(false)
                    ),
                )
            ),)
        );
    }

    #[test]
    fn conditional_if_only() {
        let script = "if (true) (42u8)";
        let result = compile_and_log(script);

        assert_instructions_equal!(
            &result,
            (
                RegularInstruction::unbounded_statements(),
                RegularInstruction::jump_if_false(2),
                RegularInstruction::True,
                RegularInstruction::uint8(42),
                RegularInstruction::unbounded_statements_end(false),
            )
        );
    }

    #[test]
    fn conditional_complex() {
        let script = "
                if (true) (
                    0u8;
                    1u8;
                )
                else if (false) (
                    0u8
                )
                else (
                    0u8
                )";
        let result = compile_and_log(script);

        assert_instructions_equal!(
            &result,
            (
                RegularInstruction::unbounded_statements(),
                RegularInstruction::jump_if_false(12),
                RegularInstruction::True,
                RegularInstruction::statements_with_children(
                    true,
                    instructions!(
                        RegularInstruction::uint8(0),
                        RegularInstruction::uint8(1)
                    )
                ),
                RegularInstruction::jump(18),
                RegularInstruction::unbounded_statements(),
                RegularInstruction::jump_if_false(7),
                RegularInstruction::False,
                RegularInstruction::uint8(0),
                RegularInstruction::jump(2),
                RegularInstruction::uint8(0),
                RegularInstruction::unbounded_statements_end(false),
                RegularInstruction::unbounded_statements_end(false),
            )
        );
    }
}
