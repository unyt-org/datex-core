use crate::ast::structs::VariableId;
use crate::compiler::error::{
    CompilerError, DetailedCompilerErrors, SimpleOrDetailedCompilerError,
    SpannedCompilerError,
};
use crate::global::dxb_block::DXBBlock;
use crate::global::operators::assignment::AssignmentOperator;
use crate::global::protocol_structures::block_header::BlockHeader;
use crate::global::protocol_structures::encrypted_header::EncryptedHeader;
use crate::global::protocol_structures::routing_header::RoutingHeader;
use core::cell::RefCell;

use crate::parser::parser_result::ValidDatexParseResult;
use crate::ast::structs::expression::{
    BinaryOperation, ComparisonOperation, DatexExpression, DatexExpressionData,
    DerefAssignment, RemoteExecution, Slot, Statements, UnaryOperation,
    UnboundedStatement, VariableAccess, VariableAssignment,
    VariableDeclaration, VariableKind,
};
use crate::compiler::context::{CompilationContext, VirtualSlot};
use crate::compiler::error::{
    DetailedCompilerErrorsWithMaybeRichAst,
    SimpleCompilerErrorOrDetailedCompilerErrorWithRichAst,
};
use crate::compiler::metadata::CompileMetadata;
use crate::compiler::scope::CompilationScope;
use crate::compiler::type_compiler::compile_type_expression;
use crate::global::instruction_codes::InstructionCode;
use crate::global::slots::InternalSlot;
use crate::libs::core::CoreLibPointerId;

use crate::core_compiler::value_compiler::{
    append_boolean, append_decimal, append_encoded_integer, append_endpoint,
    append_float_as_i16, append_float_as_i32, append_instruction_code,
    append_integer, append_text, append_typed_decimal, append_typed_integer,
    append_value_container,
};
use crate::core_compiler::value_compiler::{append_get_ref, append_key_string};
use crate::references::reference::ReferenceMutability;
use crate::runtime::execution::context::ExecutionMode;
use crate::stdlib::rc::Rc;
use crate::stdlib::vec::Vec;
use crate::utils::buffers::append_u8;
use crate::utils::buffers::append_u32;
use crate::values::core_values::decimal::Decimal;
use crate::values::pointer::PointerAddress;
use crate::values::value_container::ValueContainer;
use log::{debug, info};
use precompiler::options::PrecompilerOptions;
use precompiler::precompile_ast;
use precompiler::precompiled_ast::{AstMetadata, RichAst, VariableMetadata};
use crate::parser::Parser;
use crate::time::Instant;

pub mod context;
pub mod error;
pub mod metadata;
pub mod scope;
pub mod type_compiler;

pub mod precompiler;
#[cfg(feature = "std")]
pub mod workspace;

#[derive(Clone, Default)]
pub struct CompileOptions {
    pub compile_scope: CompilationScope,
}

impl CompileOptions {
    pub fn new_with_scope(compile_scope: CompilationScope) -> Self {
        CompileOptions {
            compile_scope,
        }
    }
}

#[derive(Debug, Clone)]
pub enum StaticValueOrDXB {
    StaticValue(Option<ValueContainer>),
    DXB(Vec<u8>),
}

impl From<Vec<u8>> for StaticValueOrDXB {
    fn from(dxb: Vec<u8>) -> Self {
        StaticValueOrDXB::DXB(dxb)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub enum VariableModel {
    /// A variable that is declared once and never reassigned afterward
    /// e.g. `const a = 42;`
    Constant,
    /// A variable that can be reassigned by updating the slot value
    /// e.g. `var a = 42; a = 69;`
    VariableSlot,
    /// A variable that can be reassigned by updating a reference value. The slot always point to this reference.
    /// When variables are transferred across realms, `VariableReference` is used for `var` variables instead of `VariableSlot`.
    /// e.g. `var a = 42; x :: (a)
    VariableReference,
}

impl From<VariableRepresentation> for VariableModel {
    fn from(value: VariableRepresentation) -> Self {
        match value {
            VariableRepresentation::Constant(_) => VariableModel::Constant,
            VariableRepresentation::VariableSlot(_) => {
                VariableModel::VariableSlot
            }
            VariableRepresentation::VariableReference { .. } => {
                VariableModel::VariableReference
            }
        }
    }
}

impl VariableModel {
    /// Determines the variable model based on the variable kind and metadata.
    pub fn infer(
        variable_kind: VariableKind,
        variable_metadata: Option<VariableMetadata>,
        execution_mode: ExecutionMode,
    ) -> Self {
        // const variables are always constant
        if variable_kind == VariableKind::Const {
            VariableModel::Constant
        }
        // for cross-realm variables, we always use VariableReference
        // if we don't know the full source text yet (e.g. in a repl), we
        // must fall back to VariableReference, because we cannot determine if
        // the variable will be transferred across realms later
        else if variable_metadata.is_none()
            || variable_metadata.unwrap().is_cross_realm
            || execution_mode.is_unbounded()
        {
            VariableModel::VariableReference
        }
        // otherwise, we use VariableSlot (default for `var` variables)
        else {
            VariableModel::VariableSlot
        }
    }

    pub fn infer_from_ast_metadata_and_type(
        ast_metadata: &AstMetadata,
        variable_id: Option<VariableId>,
        variable_kind: VariableKind,
        execution_mode: ExecutionMode,
    ) -> Self {
        let variable_metadata =
            variable_id.and_then(|id| ast_metadata.variable_metadata(id));
        Self::infer(variable_kind, variable_metadata.cloned(), execution_mode)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub enum VariableRepresentation {
    Constant(VirtualSlot),
    VariableSlot(VirtualSlot),
    VariableReference {
        /// The slot that contains the reference that is used as the variable
        variable_slot: VirtualSlot,
        /// The slot that contains the actual value container used in the script (Note: the value container may also be a reference)
        container_slot: VirtualSlot,
    },
}

/// Represents a variable in the DATEX script.
#[derive(Debug, Clone)]
pub struct Variable {
    pub name: String,
    pub kind: VariableKind,
    pub representation: VariableRepresentation,
}

impl Variable {
    pub fn new_const(name: String, slot: VirtualSlot) -> Self {
        Variable {
            name,
            kind: VariableKind::Const,
            representation: VariableRepresentation::Constant(slot),
        }
    }

    pub fn new_variable_slot(
        name: String,
        kind: VariableKind,
        slot: VirtualSlot,
    ) -> Self {
        Variable {
            name,
            kind,
            representation: VariableRepresentation::VariableSlot(slot),
        }
    }

    pub fn new_variable_reference(
        name: String,
        kind: VariableKind,
        variable_slot: VirtualSlot,
        container_slot: VirtualSlot,
    ) -> Self {
        Variable {
            name,
            kind,
            representation: VariableRepresentation::VariableReference {
                variable_slot,
                container_slot,
            },
        }
    }

    pub fn slots(&self) -> Vec<VirtualSlot> {
        match &self.representation {
            VariableRepresentation::Constant(slot) => vec![*slot],
            VariableRepresentation::VariableSlot(slot) => vec![*slot],
            VariableRepresentation::VariableReference {
                variable_slot,
                container_slot,
            } => {
                vec![*variable_slot, *container_slot]
            }
        }
    }
}

/// Compiles a DATEX script text into a single DXB block including routing and block headers.
/// This function is used to create a block that can be sent over the network.
pub fn compile_block(
    datex_script: &str,
) -> Result<Vec<u8>, SimpleOrDetailedCompilerError> {
    let (body, _) = compile_script(datex_script, CompileOptions::default())?;

    let routing_header = RoutingHeader::default();

    let block_header = BlockHeader::default();
    let encrypted_header = EncryptedHeader::default();

    let block =
        DXBBlock::new(routing_header, block_header, encrypted_header, body);

    let bytes = block
        .to_bytes()
        .map_err(|e| CompilerError::SerializationError)?;
    Ok(bytes)
}

/// Compiles a DATEX script text into a DXB body
pub fn compile_script(
    datex_script: &str,
    options: CompileOptions,
) -> Result<(Vec<u8>, CompilationScope), SpannedCompilerError> {
    compile_template(datex_script, &[], options)
}

/// Directly extracts a static value from a DATEX script as a `ValueContainer`.
/// This only works if the script does not contain any dynamic values or operations.
/// All JSON-files can be compiled to static values, but not all DATEX scripts.
pub fn extract_static_value_from_script(
    datex_script: &str,
) -> Result<Option<ValueContainer>, SpannedCompilerError> {
    let valid_parse_result = Parser::parse(datex_script)?;
    extract_static_value_from_ast(&valid_parse_result)
        .map(Some)
        .map_err(SpannedCompilerError::from)
}

/// Converts a DATEX script template text with inserted values into an AST with metadata
/// If the script does not contain any dynamic values or operations, the static result value is
/// directly returned instead of the AST.
pub fn compile_script_or_return_static_value<'a>(
    datex_script: &'a str,
    mut options: CompileOptions,
) -> Result<(StaticValueOrDXB, CompilationScope), SpannedCompilerError> {
    let ast = parse_datex_script_to_rich_ast_simple_error(
        datex_script,
        &mut options,
    )?;
    let mut compilation_context = CompilationContext::new(
        Vec::with_capacity(256),
        vec![],
        options.compile_scope.execution_mode,
    );
    // FIXME #480: no clone here
    let scope = compile_ast(ast.clone(), &mut compilation_context, options)?;
    if compilation_context.has_non_static_value {
        Ok((StaticValueOrDXB::DXB(compilation_context.buffer), scope))
    } else {
        // try to extract static value from AST
        extract_static_value_from_ast(&ast.ast)
            .map(|value| (StaticValueOrDXB::StaticValue(Some(value)), scope))
            .map_err(SpannedCompilerError::from)
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
    }) = &mut ast.data
    {
        *unbounded = unbounded_section;
        *is_terminated
    } else {
        // wrap in statements
        let original_ast = ast.clone();
        ast.data = DatexExpressionData::Statements(Statements {
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
    let parse_start = Instant::now();
    let mut valid_parse_result = Parser::parse(datex_script)?;

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
            valid_parse_result.data,
            DatexExpressionData::Statements(Statements {
                is_terminated: true,
                ..
            })
        )
    };
    debug!(
        " [parse took {} ms]",
        parse_start.elapsed().as_millis()
    );
    let precompile_start = Instant::now();
    let res = precompile_to_rich_ast(
        valid_parse_result,
        &mut options.compile_scope,
        PrecompilerOptions {
            detailed_errors: false,
        },
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
) -> Result<RichAst, DetailedCompilerErrorsWithMaybeRichAst> {
    let (ast, parser_errors) = Parser::parse_collecting(datex_script).into_ast_and_errors();
    precompile_to_rich_ast(
        ast,
        &mut options.compile_scope,
        PrecompilerOptions {
            detailed_errors: true,
        },
    )
    .map_err(|e| match e {
        SimpleCompilerErrorOrDetailedCompilerErrorWithRichAst::Detailed(mut e) => {
            // append parser errors to detailed errors
            e.errors.errors.extend(parser_errors.into_iter().map(SpannedCompilerError::from));
            e.into()
        }
        _ => unreachable!(), // because detailed_errors: true
    })
}

/// Compiles a DATEX script template text with inserted values into a DXB body
pub fn compile_template(
    datex_script: &str,
    inserted_values: &[ValueContainer],
    mut options: CompileOptions,
) -> Result<(Vec<u8>, CompilationScope), SpannedCompilerError> {
    let ast = parse_datex_script_to_rich_ast_simple_error(
        datex_script,
        &mut options,
    )?;
    let mut compilation_context = CompilationContext::new(
        Vec::with_capacity(256),
        // TODO #482: no clone here
        inserted_values.to_vec(),
        options.compile_scope.execution_mode,
    );
    let compile_start = Instant::now();
    let res = compile_ast(ast, &mut compilation_context, options)
        .map(|scope| (compilation_context.buffer, scope))
        .map_err(SpannedCompilerError::from);
    debug!(
        " [compile_ast took {} ms]",
        compile_start.elapsed().as_millis()
    );
    res
}

/// Compiles a precompiled DATEX AST, returning the compilation context and scope
fn compile_ast(
    ast: RichAst,
    compilation_context: &mut CompilationContext,
    options: CompileOptions,
) -> Result<CompilationScope, CompilerError> {
    let compilation_scope =
        compile_rich_ast(compilation_context, ast, options.compile_scope)?;
    Ok(compilation_scope)
}

/// Tries to extract a static value from a DATEX expression AST.
/// If the expression is not a static value (e.g., contains a placeholder or dynamic operation),
/// it returns an error.
fn extract_static_value_from_ast(
    ast: &DatexExpression,
) -> Result<ValueContainer, CompilerError> {
    if let DatexExpressionData::Placeholder = ast.data {
        return Err(CompilerError::NonStaticValue);
    }
    ValueContainer::try_from(&ast.data)
        .map_err(|_| CompilerError::NonStaticValue)
}

/// Macro for compiling a DATEX script template text with inserted values into a DXB body,
/// behaves like the format! macro.
/// Example:
/// ```
/// use datex_core::compile;
/// compile!("4 + ?", 42);
/// compile!("? + ?", 1, 2);
#[macro_export]
macro_rules! compile {
    ($fmt:literal $(, $arg:expr )* $(,)?) => {
        {
            let script: &str = $fmt.into();
            let values: &[$crate::values::value_container::ValueContainer] = &[$($arg.into()),*];

            $crate::compiler::compile_template(&script, values, $crate::compiler::CompileOptions::default())
        }
    }
}

/// Precompiles a DATEX expression AST into an AST with metadata.
fn precompile_to_rich_ast(
    valid_parse_result: DatexExpression,
    scope: &mut CompilationScope,
    precompiler_options: PrecompilerOptions,
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
        )?
    } else {
        // if no precompiler data, just use the AST with default metadata
        RichAst::new_without_metadata(valid_parse_result)
    };

    Ok(rich_ast)
}

pub fn compile_rich_ast(
    compilation_context: &mut CompilationContext,
    rich_ast: RichAst,
    scope: CompilationScope,
) -> Result<CompilationScope, CompilerError> {
    let scope = compile_expression(
        compilation_context,
        rich_ast,
        CompileMetadata::outer(),
        scope,
    )?;

    // handle scope virtual addr mapping
    compilation_context.remap_virtual_slots();
    Ok(scope)
}

fn compile_expression(
    compilation_context: &mut CompilationContext,
    rich_ast: RichAst,
    meta: CompileMetadata,
    mut scope: CompilationScope,
) -> Result<CompilationScope, CompilerError> {
    let metadata = rich_ast.metadata;
    // TODO #483: no clone
    match rich_ast.ast.data.clone() {
        DatexExpressionData::Integer(int) => {
            append_integer(&mut compilation_context.buffer, &int);
        }
        DatexExpressionData::TypedInteger(typed_int) => {
            append_encoded_integer(&mut compilation_context.buffer, &typed_int);
        }
        DatexExpressionData::Decimal(decimal) => match &decimal {
            Decimal::Finite(big_decimal) if big_decimal.is_integer() => {
                if let Some(int) = big_decimal.to_i16() {
                    append_float_as_i16(&mut compilation_context.buffer, int);
                } else if let Some(int) = big_decimal.to_i32() {
                    append_float_as_i32(&mut compilation_context.buffer, int);
                } else {
                    append_decimal(&mut compilation_context.buffer, &decimal);
                }
            }
            _ => {
                append_decimal(&mut compilation_context.buffer, &decimal);
            }
        },
        DatexExpressionData::TypedDecimal(typed_decimal) => {
            append_typed_decimal(
                &mut compilation_context.buffer,
                &typed_decimal,
            );
        }
        DatexExpressionData::Text(text) => {
            append_text(&mut compilation_context.buffer, &text);
        }
        DatexExpressionData::Boolean(boolean) => {
            append_boolean(&mut compilation_context.buffer, boolean);
        }
        DatexExpressionData::Endpoint(endpoint) => {
            append_endpoint(&mut compilation_context.buffer, &endpoint);
        }
        DatexExpressionData::Null => {
            append_instruction_code(
                &mut compilation_context.buffer,
                InstructionCode::NULL,
            );
        }
        DatexExpressionData::List(list) => {
            match list.items.len() {
                0..=255 => {
                    compilation_context
                        .append_instruction_code(InstructionCode::SHORT_LIST);
                    append_u8(
                        &mut compilation_context.buffer,
                        list.items.len() as u8,
                    );
                }
                _ => {
                    compilation_context
                        .append_instruction_code(InstructionCode::LIST);
                    append_u32(
                        &mut compilation_context.buffer,
                        list.items.len() as u32, // FIXME: conversion from usize to u32
                    );
                }
            }
            for item in list.items {
                scope = compile_expression(
                    compilation_context,
                    RichAst::new(item, &metadata),
                    CompileMetadata::default(),
                    scope,
                )?;
            }
        }
        DatexExpressionData::Map(map) => {
            // TODO #434: Handle string keyed maps (structs)
            match map.entries.len() {
                0..=255 => {
                    compilation_context
                        .append_instruction_code(InstructionCode::SHORT_MAP);
                    append_u8(
                        &mut compilation_context.buffer,
                        map.entries.len() as u8,
                    );
                }
                _ => {
                    compilation_context
                        .append_instruction_code(InstructionCode::MAP);
                    append_u32(
                        &mut compilation_context.buffer,
                        map.entries.len() as u32, // FIXME: conversion from usize to u32
                    );
                }
            }
            for (key, value) in map.entries {
                scope = compile_key_value_entry(
                    compilation_context,
                    key,
                    value,
                    &metadata,
                    scope,
                )?;
            }
        }
        DatexExpressionData::Placeholder => {
            append_value_container(
                &mut compilation_context.buffer,
                compilation_context
                    .inserted_values
                    .get(compilation_context.inserted_value_index)
                    .unwrap(),
            );
            compilation_context.inserted_value_index += 1;
        }

        // statements
        DatexExpressionData::Statements(Statements {
            mut statements,
            is_terminated,
            unbounded,
        }) => {
            compilation_context.mark_has_non_static_value();
            // if single statement and not terminated, just compile the expression
            // (not for unbounded execution mode)
            if unbounded.is_none() && statements.len() == 1 && !is_terminated {
                scope = compile_expression(
                    compilation_context,
                    RichAst::new(statements.remove(0), &metadata),
                    CompileMetadata::default(),
                    scope,
                )?;
            } else {
                let is_outer_context = meta.is_outer_context();

                // if not outer context, new scope
                let mut child_scope = if is_outer_context {
                    scope
                } else {
                    scope.push()
                };

                if let Some(UnboundedStatement { is_first, .. }) = unbounded {
                    // if this is the first section of an unbounded statements block, mark as unbounded
                    if is_first {
                        compilation_context.append_instruction_code(
                            InstructionCode::UNBOUNDED_STATEMENTS,
                        );
                    }
                    // if not first, don't insert any instruction code
                }
                // otherwise, statements with fixed length
                else {
                    let len = statements.len();

                    match len {
                        0..=255 => {
                            compilation_context.append_instruction_code(
                                InstructionCode::SHORT_STATEMENTS,
                            );
                            append_u8(
                                &mut compilation_context.buffer,
                                len as u8,
                            );
                        }
                        _ => {
                            compilation_context.append_instruction_code(
                                InstructionCode::STATEMENTS,
                            );
                            append_u32(
                                &mut compilation_context.buffer,
                                len as u32, // FIXME: conversion from usize to u32
                            );
                        }
                    }

                    // append termination flag
                    append_u8(
                        &mut compilation_context.buffer,
                        if is_terminated { 1 } else { 0 },
                    );
                }

                for (i, statement) in statements.into_iter().enumerate() {
                    child_scope = compile_expression(
                        compilation_context,
                        RichAst::new(statement, &metadata),
                        CompileMetadata::default(),
                        child_scope,
                    )?;
                }
                if !meta.is_outer_context() {
                    let scope_data = child_scope
                        .pop()
                        .ok_or(CompilerError::ScopePopError)?;
                    scope = scope_data.0; // set parent scope
                    // drop all slot addresses that were allocated in this scope
                    for slot_address in scope_data.1 {
                        compilation_context.append_instruction_code(
                            InstructionCode::DROP_SLOT,
                        );
                        // insert virtual slot address for dropping
                        compilation_context
                            .insert_virtual_slot_address(slot_address);
                    }
                } else {
                    scope = child_scope;
                }

                // if this is the last section of an unbounded statements block, add closing instruction
                if let Some(UnboundedStatement { is_last: true, .. }) =
                    unbounded
                {
                    compilation_context.append_instruction_code(
                        InstructionCode::UNBOUNDED_STATEMENTS_END,
                    );
                    // append termination flag
                    append_u8(
                        &mut compilation_context.buffer,
                        if is_terminated { 1 } else { 0 },
                    );
                }
            }
        }

        // unary operations (negation, not, etc.)
        DatexExpressionData::UnaryOperation(UnaryOperation {
            operator,
            expression,
        }) => {
            compilation_context
                .append_instruction_code(InstructionCode::from(&operator));
            scope = compile_expression(
                compilation_context,
                RichAst::new(*expression, &metadata),
                CompileMetadata::default(),
                scope,
            )?;
        }

        // operations (add, subtract, multiply, divide, etc.)
        DatexExpressionData::BinaryOperation(BinaryOperation {
            operator,
            left,
            right,
            ..
        }) => {
            compilation_context.mark_has_non_static_value();
            // append binary code for operation if not already current binary operator
            compilation_context
                .append_instruction_code(InstructionCode::from(&operator));
            scope = compile_expression(
                compilation_context,
                RichAst::new(*left, &metadata),
                CompileMetadata::default(),
                scope,
            )?;
            scope = compile_expression(
                compilation_context,
                RichAst::new(*right, &metadata),
                CompileMetadata::default(),
                scope,
            )?;
        }

        // comparisons (e.g., equal, not equal, greater than, etc.)
        DatexExpressionData::ComparisonOperation(ComparisonOperation {
            operator,
            left,
            right,
        }) => {
            compilation_context.mark_has_non_static_value();
            // append binary code for operation if not already current binary operator
            compilation_context
                .append_instruction_code(InstructionCode::from(&operator));
            scope = compile_expression(
                compilation_context,
                RichAst::new(*left, &metadata),
                CompileMetadata::default(),
                scope,
            )?;
            scope = compile_expression(
                compilation_context,
                RichAst::new(*right, &metadata),
                CompileMetadata::default(),
                scope,
            )?;
        }

        // apply
        DatexExpressionData::Apply(apply) => {
            compilation_context.mark_has_non_static_value();
            let base_expression = apply.base;
            scope = compile_expression(
                compilation_context,
                RichAst::new(*base_expression, &metadata),
                CompileMetadata::default(),
                scope,
            )?;
            // TODO: apply
        }

        DatexExpressionData::PropertyAccess(property_access) => {
            todo!()
        }

        DatexExpressionData::GenericInstantiation(generic_instantiation) => {
            // NOTE: might already be handled in type compilation
            todo!()
        }

        // variables
        // declaration
        DatexExpressionData::VariableDeclaration(VariableDeclaration {
            id,
            name,
            kind,
            type_annotation,
            init_expression: value,
        }) => {
            compilation_context.mark_has_non_static_value();

            // allocate new slot for variable
            let virtual_slot_addr = scope.get_next_virtual_slot();
            compilation_context
                .append_instruction_code(InstructionCode::ALLOCATE_SLOT);
            compilation_context.insert_virtual_slot_address(
                VirtualSlot::local(virtual_slot_addr),
            );
            // compile expression
            scope = compile_expression(
                compilation_context,
                RichAst::new(*value, &metadata),
                CompileMetadata::default(),
                scope,
            )?;

            let variable_model =
                VariableModel::infer_from_ast_metadata_and_type(
                    &metadata.borrow(),
                    id,
                    kind,
                    compilation_context.execution_mode,
                );

            // create new variable depending on the model
            let variable = match variable_model {
                VariableModel::VariableReference => {
                    // allocate an additional slot with a reference to the variable
                    let virtual_slot_addr_for_var =
                        scope.get_next_virtual_slot();
                    compilation_context.append_instruction_code(
                        InstructionCode::ALLOCATE_SLOT,
                    );
                    compilation_context.insert_virtual_slot_address(
                        VirtualSlot::local(virtual_slot_addr_for_var),
                    );
                    // indirect reference to the variable
                    compilation_context
                        .append_instruction_code(InstructionCode::CREATE_REF);
                    // append binary code to load variable
                    compilation_context
                        .append_instruction_code(InstructionCode::GET_SLOT);
                    compilation_context.insert_virtual_slot_address(
                        VirtualSlot::local(virtual_slot_addr),
                    );

                    Variable::new_variable_reference(
                        name.clone(),
                        kind,
                        VirtualSlot::local(virtual_slot_addr_for_var),
                        VirtualSlot::local(virtual_slot_addr),
                    )
                }
                VariableModel::Constant => Variable::new_const(
                    name.clone(),
                    VirtualSlot::local(virtual_slot_addr),
                ),
                VariableModel::VariableSlot => Variable::new_variable_slot(
                    name.clone(),
                    kind,
                    VirtualSlot::local(virtual_slot_addr),
                ),
            };

            scope.register_variable_slot(variable);
        }

        DatexExpressionData::GetReference(address) => {
            compilation_context.mark_has_non_static_value();
            append_get_ref(&mut compilation_context.buffer, &address)
        }

        // assignment
        DatexExpressionData::VariableAssignment(VariableAssignment {
            operator,
            name,
            expression,
            ..
        }) => {
            compilation_context.mark_has_non_static_value();
            // get variable slot address
            let (virtual_slot, kind) = scope
                .resolve_variable_name_to_virtual_slot(&name)
                .ok_or_else(|| {
                    CompilerError::UndeclaredVariable(name.clone())
                })?;

            // TODO #484: check not needed, is already handled in precompiler - can we guarantee this?
            // if const, return error
            if kind == VariableKind::Const {
                return Err(CompilerError::AssignmentToConst(name.clone()));
            }

            match operator {
                AssignmentOperator::Assign => {
                    // append binary code to load variable
                    info!(
                        "append variable virtual slot: {virtual_slot:?}, name: {name}"
                    );
                    compilation_context
                        .append_instruction_code(InstructionCode::SET_SLOT);
                    // compilation_context.append_instruction_code(
                    //     InstructionCode::from(&operator),
                    // );
                }
                AssignmentOperator::AddAssign
                | AssignmentOperator::SubtractAssign => {
                    // TODO #435: handle mut type
                    // // if immutable reference, return error
                    // if mut_type == Some(ReferenceMutability::Immutable) {
                    //     return Err(
                    //         CompilerError::AssignmentToImmutableReference(
                    //             name.clone(),
                    //         ),
                    //     );
                    // }
                    // // if immutable value, return error
                    // else if mut_type == None {
                    //     return Err(CompilerError::AssignmentToImmutableValue(
                    //         name.clone(),
                    //     ));
                    // }
                    compilation_context
                        .append_instruction_code(InstructionCode::SET_SLOT);
                    compilation_context.append_instruction_code(
                        InstructionCode::from(&operator),
                    );
                }
                op => core::todo!("#436 Handle assignment operator: {op:?}"),
            }

            compilation_context.insert_virtual_slot_address(virtual_slot);
            // compile expression
            scope = compile_expression(
                compilation_context,
                RichAst::new(*expression, &metadata),
                CompileMetadata::default(),
                scope,
            )?;
        }

        DatexExpressionData::DerefAssignment(DerefAssignment {
            operator,
            deref_expression,
            assigned_expression,
        }) => {
            compilation_context.mark_has_non_static_value();

            compilation_context
                .append_instruction_code(InstructionCode::SET_REFERENCE_VALUE);

            compilation_context
                .append_instruction_code(InstructionCode::from(&operator));

            // compile deref expression
            scope = compile_expression(
                compilation_context,
                RichAst::new(*deref_expression, &metadata),
                CompileMetadata::default(),
                scope,
            )?;

            // compile assigned expression
            scope = compile_expression(
                compilation_context,
                RichAst::new(*assigned_expression, &metadata),
                CompileMetadata::default(),
                scope,
            )?;
        }

        // variable access
        DatexExpressionData::VariableAccess(VariableAccess {
            name, ..
        }) => {
            compilation_context.mark_has_non_static_value();
            // get variable slot address
            let (virtual_slot, ..) = scope
                .resolve_variable_name_to_virtual_slot(&name)
                .ok_or_else(|| {
                    CompilerError::UndeclaredVariable(name.clone())
                })?;
            // append binary code to load variable
            compilation_context
                .append_instruction_code(InstructionCode::GET_SLOT);
            compilation_context.insert_virtual_slot_address(virtual_slot);
        }

        // remote execution
        DatexExpressionData::RemoteExecution(RemoteExecution {
            left: caller,
            right: script,
        }) => {
            compilation_context.mark_has_non_static_value();

            // insert remote execution code
            compilation_context
                .append_instruction_code(InstructionCode::REMOTE_EXECUTION);

            // compile remote execution block
            let mut execution_block_ctx = CompilationContext::new(
                Vec::with_capacity(256),
                vec![],
                ExecutionMode::Static,
            );
            let external_scope = compile_rich_ast(
                &mut execution_block_ctx,
                RichAst::new(*script, &metadata),
                CompilationScope::new_with_external_parent_scope(scope),
            )?;
            // reset to current scope
            scope = external_scope
                .pop_external()
                .ok_or_else(|| CompilerError::ScopePopError)?;

            let external_slots = execution_block_ctx.external_slots();

            // --- start block
            // set block size (len of compilation_context.buffer)
            append_u32(
                &mut compilation_context.buffer,
                execution_block_ctx.buffer.len() as u32,
            );
            // set injected slot count
            append_u32(
                &mut compilation_context.buffer,
                external_slots.len() as u32,
            );
            for slot in external_slots {
                compilation_context.insert_virtual_slot_address(slot.upgrade());
            }

            // insert block body (compilation_context.buffer)
            compilation_context
                .buffer
                .extend_from_slice(&execution_block_ctx.buffer);
            // --- end block

            // insert compiled caller expression
            scope = compile_expression(
                compilation_context,
                RichAst::new(*caller, &metadata),
                CompileMetadata::default(),
                scope,
            )?;
        }

        // named slot
        DatexExpressionData::Slot(Slot::Named(name)) => {
            match name.as_str() {
                "endpoint" => {
                    compilation_context
                        .append_instruction_code(InstructionCode::GET_SLOT);
                    append_u32(
                        &mut compilation_context.buffer,
                        InternalSlot::ENDPOINT as u32,
                    );
                }
                "core" => append_get_ref(
                    &mut compilation_context.buffer,
                    &PointerAddress::from(CoreLibPointerId::Core),
                ),
                _ => {
                    // invalid slot name
                    return Err(CompilerError::InvalidSlotName(name.clone()));
                }
            }
        }

        // pointer address
        DatexExpressionData::PointerAddress(address) => {
            append_get_ref(&mut compilation_context.buffer, &address);
        }

        // refs
        DatexExpressionData::CreateRef(create_ref) => {
            compilation_context.mark_has_non_static_value();
            compilation_context.append_instruction_code(
                match create_ref.mutability {
                    ReferenceMutability::Immutable => {
                        InstructionCode::CREATE_REF
                    }
                    ReferenceMutability::Mutable => {
                        InstructionCode::CREATE_REF_MUT
                    }
                },
            );
            scope = compile_expression(
                compilation_context,
                RichAst::new(*create_ref.expression, &metadata),
                CompileMetadata::default(),
                scope,
            )?;
        }

        DatexExpressionData::Type(type_expression) => {
            compilation_context
                .append_instruction_code(InstructionCode::TYPE_EXPRESSION);
            scope = compile_type_expression(
                compilation_context,
                &type_expression,
                metadata,
                scope,
            )?;
        }

        DatexExpressionData::Deref(deref) => {
            compilation_context.mark_has_non_static_value();
            compilation_context.append_instruction_code(InstructionCode::DEREF);
            scope = compile_expression(
                compilation_context,
                RichAst::new(*deref.expression, &metadata),
                CompileMetadata::default(),
                scope,
            )?;
        }

        e => {
            println!("Unhandled expression in compiler: {:?}", e);
            return Err(CompilerError::UnexpectedTerm(Box::new(rich_ast.ast)));
        }
    }

    Ok(scope)
}

fn compile_key_value_entry(
    compilation_scope: &mut CompilationContext,
    key: DatexExpression,
    value: DatexExpression,
    metadata: &Rc<RefCell<AstMetadata>>,
    mut scope: CompilationScope,
) -> Result<CompilationScope, CompilerError> {
    match key.data {
        // text -> insert key string
        DatexExpressionData::Text(text) => {
            append_key_string(&mut compilation_scope.buffer, &text);
        }
        // other -> insert key as dynamic
        _ => {
            compilation_scope
                .append_instruction_code(InstructionCode::KEY_VALUE_DYNAMIC);
            scope = compile_expression(
                compilation_scope,
                RichAst::new(key, metadata),
                CompileMetadata::default(),
                scope,
            )?;
        }
    };
    // insert value
    scope = compile_expression(
        compilation_scope,
        RichAst::new(value, metadata),
        CompileMetadata::default(),
        scope,
    )?;
    Ok(scope)
}

#[cfg(test)]
pub mod tests {
    use super::{
        CompilationContext, CompileOptions, StaticValueOrDXB, compile_ast,
        compile_script, compile_script_or_return_static_value,
        compile_template, parse_datex_script_to_rich_ast_simple_error,
    };
    use crate::stdlib::assert_matches::assert_matches;
    use crate::stdlib::io::Read;
    use crate::stdlib::vec;

    use crate::compiler::scope::CompilationScope;
    use crate::global::type_instruction_codes::TypeInstructionCode;
    use crate::libs::core::CoreLibPointerId;
    use crate::runtime::execution::ExecutionError;
    use crate::runtime::execution::context::{
        ExecutionContext, ExecutionMode, LocalExecutionContext,
    };
    use crate::values::core_values::integer::Integer;
    use crate::values::pointer::PointerAddress;
    use crate::values::value_container::ValueContainer;
    use crate::{
        global::instruction_codes::InstructionCode, logger::init_logger_debug,
    };
    use datex_core::compiler::error::CompilerError;
    use datex_core::values::core_values::integer::typed_integer::TypedInteger;
    use log::*;

    fn compile_and_log(datex_script: &str) -> Vec<u8> {
        init_logger_debug();
        let (result, _) =
            compile_script(datex_script, CompileOptions::default()).unwrap();
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

    fn get_compilation_context(script: &str) -> CompilationContext {
        let mut options = CompileOptions::default();
        let ast =
            parse_datex_script_to_rich_ast_simple_error(script, &mut options)
                .unwrap();

        let mut compilation_context = CompilationContext::new(
            Vec::with_capacity(256),
            vec![],
            options.compile_scope.execution_mode,
        );
        compile_ast(ast, &mut compilation_context, options).unwrap();
        compilation_context
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
                let (dxb, new_compilation_scope) = compile_script(
                    script_part,
                    CompileOptions::new_with_scope(compilation_scope),
                )
                .unwrap();
                compilation_scope = new_compilation_scope;
                yield dxb;
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
        init_logger_debug();

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
        init_logger_debug();

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
    fn is_operator() {
        init_logger_debug();

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
            "const a = &mut 42u8; const b = &mut 69u8; a is b".to_string(); // a is b
        let result = compile_and_log(&datex_script);
        assert_eq!(
            result,
            vec![
                InstructionCode::SHORT_STATEMENTS.into(),
                3,
                0, // not terminated
                InstructionCode::ALLOCATE_SLOT.into(),
                0,
                0,
                0,
                0,
                InstructionCode::CREATE_REF_MUT.into(),
                InstructionCode::UINT_8.into(),
                42,
                // val b = 69;
                InstructionCode::ALLOCATE_SLOT.into(),
                1,
                0,
                0,
                0,
                InstructionCode::CREATE_REF_MUT.into(),
                InstructionCode::UINT_8.into(),
                69,
                // a is b
                InstructionCode::IS.into(),
                InstructionCode::GET_SLOT.into(),
                0,
                0,
                0,
                0, // slot address for a
                InstructionCode::GET_SLOT.into(),
                1,
                0,
                0,
                0, // slot address for b
            ]
        );
    }

    #[test]
    fn equality_operator() {
        init_logger_debug();

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
        init_logger_debug();

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
        init_logger_debug();

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
        init_logger_debug();

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
        init_logger_debug();

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
        init_logger_debug();

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
        init_logger_debug();
        let val = 42;
        let datex_script = format!("{val}u8"); // 42
        let result = compile_and_log(&datex_script);
        assert_eq!(result, vec![InstructionCode::UINT_8.into(), val,]);
    }

    // Test for decimal
    #[test]
    fn decimal() {
        init_logger_debug();
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
        init_logger_debug();
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
        init_logger_debug();
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
        init_logger_debug();
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
        init_logger_debug();
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
        init_logger_debug();
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
        init_logger_debug();
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
        init_logger_debug();
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
        init_logger_debug();
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
        init_logger_debug();
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
        init_logger_debug();
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
        init_logger_debug();
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
        init_logger_debug();
        let datex_script = "{}";
        let result = compile_and_log(datex_script);
        let expected: Vec<u8> = vec![
            InstructionCode::SHORT_MAP.into(),
            0, // length
        ];
        assert_eq!(result, expected);
    }

    #[test]
    fn allocate_slot() {
        init_logger_debug();
        let script = "const a = 42u8";
        let result = compile_and_log(script);
        assert_eq!(
            result,
            vec![
                InstructionCode::ALLOCATE_SLOT.into(),
                // slot index as u32
                0,
                0,
                0,
                0,
                InstructionCode::UINT_8.into(),
                42,
            ]
        );
    }

    #[test]
    fn allocate_slot_with_value() {
        init_logger_debug();
        let script = "const a = 42u8; a + 1u8";
        let result = compile_and_log(script);
        assert_eq!(
            result,
            vec![
                InstructionCode::SHORT_STATEMENTS.into(),
                2,
                0, // not terminated
                InstructionCode::ALLOCATE_SLOT.into(),
                // slot index as u32
                0,
                0,
                0,
                0,
                InstructionCode::UINT_8.into(),
                42,
                InstructionCode::ADD.into(),
                InstructionCode::GET_SLOT.into(),
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
    fn allocate_scoped_slots() {
        init_logger_debug();
        let script = "const a = 42u8; (const a = 43u8; a); a";
        let result = compile_and_log(script);
        assert_eq!(
            result,
            vec![
                InstructionCode::SHORT_STATEMENTS.into(),
                3,
                0, // not terminated
                InstructionCode::ALLOCATE_SLOT.into(),
                0,
                0,
                0,
                0,
                InstructionCode::UINT_8.into(),
                42,
                InstructionCode::SHORT_STATEMENTS.into(),
                2,
                0, // not terminated
                InstructionCode::ALLOCATE_SLOT.into(),
                1,
                0,
                0,
                0,
                InstructionCode::UINT_8.into(),
                43,
                InstructionCode::GET_SLOT.into(),
                1,
                0,
                0,
                0,
                InstructionCode::DROP_SLOT.into(),
                1,
                0,
                0,
                0,
                InstructionCode::GET_SLOT.into(),
                // slot index as u32
                0,
                0,
                0,
                0,
            ]
        );
    }

    #[test]
    fn allocate_scoped_slots_with_parent_variables() {
        init_logger_debug();
        let script =
            "const a = 42u8; const b = 41u8; (const a = 43u8; a; b); a";
        let result = compile_and_log(script);
        assert_eq!(
            result,
            vec![
                InstructionCode::SHORT_STATEMENTS.into(),
                4,
                0, // not terminated
                InstructionCode::ALLOCATE_SLOT.into(),
                0,
                0,
                0,
                0,
                InstructionCode::UINT_8.into(),
                42,
                InstructionCode::ALLOCATE_SLOT.into(),
                1,
                0,
                0,
                0,
                InstructionCode::UINT_8.into(),
                41,
                InstructionCode::SHORT_STATEMENTS.into(),
                3,
                0, // not terminated
                InstructionCode::ALLOCATE_SLOT.into(),
                2,
                0,
                0,
                0,
                InstructionCode::UINT_8.into(),
                43,
                InstructionCode::GET_SLOT.into(),
                2,
                0,
                0,
                0,
                InstructionCode::GET_SLOT.into(),
                1,
                0,
                0,
                0,
                InstructionCode::DROP_SLOT.into(),
                2,
                0,
                0,
                0,
                InstructionCode::GET_SLOT.into(),
                // slot index as u32
                0,
                0,
                0,
                0,
            ]
        );
    }

    #[test]
    fn allocate_ref() {
        init_logger_debug();
        let script = "const a = &mut 42u8";
        let result = compile_and_log(script);
        assert_eq!(
            result,
            vec![
                InstructionCode::ALLOCATE_SLOT.into(),
                // slot index as u32
                0,
                0,
                0,
                0,
                InstructionCode::CREATE_REF_MUT.into(),
                InstructionCode::UINT_8.into(),
                42,
            ]
        );
    }

    #[test]
    fn read_ref() {
        init_logger_debug();
        let script = "const a = &mut 42u8; a";
        let result = compile_and_log(script);
        assert_eq!(
            result,
            vec![
                InstructionCode::SHORT_STATEMENTS.into(),
                2,
                0, // not terminated
                InstructionCode::ALLOCATE_SLOT.into(),
                // slot index as u32
                0,
                0,
                0,
                0,
                InstructionCode::CREATE_REF_MUT.into(),
                InstructionCode::UINT_8.into(),
                42,
                InstructionCode::GET_SLOT.into(),
                // slot index as u32
                0,
                0,
                0,
                0,
            ]
        );
    }

    #[test]
    fn compile() {
        init_logger_debug();
        let result = compile_template(
            "? + ?",
            &[
                TypedInteger::from(1u8).into(),
                TypedInteger::from(2u8).into(),
            ],
            CompileOptions::default(),
        );
        assert_eq!(
            result.unwrap().0,
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
        init_logger_debug();
        let a = TypedInteger::from(1u8);
        let result = compile!("?", a);
        assert_eq!(result.unwrap().0, vec![InstructionCode::UINT_8.into(), 1,]);
    }

    #[test]
    fn compile_macro_multi() {
        init_logger_debug();
        let result =
            compile!("? + ?", TypedInteger::from(1u8), TypedInteger::from(2u8));
        assert_eq!(
            result.unwrap().0,
            vec![
                InstructionCode::ADD.into(),
                InstructionCode::UINT_8.into(),
                1,
                InstructionCode::UINT_8.into(),
                2
            ]
        );
    }

    fn get_json_test_string(file_path: &str) -> String {
        // read json from test file
        let file_path = format!("benches/json/{file_path}");
        let file_path = std::path::Path::new(&file_path);
        let file =
            std::fs::File::open(file_path).expect("Failed to open test.json");
        let mut reader = crate::stdlib::io::BufReader::new(file);
        let mut json_string = String::new();
        reader
            .read_to_string(&mut json_string)
            .expect("Failed to read test.json");
        json_string
    }

    #[test]
    #[ignore = "Parser issues chumsky"]
    fn json_to_dxb_large_file() {
        let json = get_json_test_string("test2.json");
        let _ = compile_script(&json, CompileOptions::default())
            .expect("Failed to parse JSON string");
    }

    #[test]
    fn static_value_detection() {
        init_logger_debug();

        // non-static
        let script = "1 + 2";
        let compilation_scope = get_compilation_context(script);
        assert!(compilation_scope.has_non_static_value);

        let script = "1 2";
        let compilation_scope = get_compilation_context(script);
        assert!(compilation_scope.has_non_static_value);

        let script = "1;2";
        let compilation_scope = get_compilation_context(script);
        assert!(compilation_scope.has_non_static_value);

        let script = r#"{("x" + "y"): 1}"#;
        let compilation_scope = get_compilation_context(script);
        assert!(compilation_scope.has_non_static_value);

        // static
        let script = "1";
        let compilation_scope = get_compilation_context(script);
        assert!(!compilation_scope.has_non_static_value);

        let script = "[]";
        let compilation_scope = get_compilation_context(script);
        assert!(!compilation_scope.has_non_static_value);

        let script = "{}";
        let compilation_scope = get_compilation_context(script);
        assert!(!compilation_scope.has_non_static_value);

        let script = "[1,2,3]";
        let compilation_scope = get_compilation_context(script);
        assert!(!compilation_scope.has_non_static_value);

        let script = "{a: 2}";
        let compilation_scope = get_compilation_context(script);
        assert!(!compilation_scope.has_non_static_value);

        // because of unary - 42
        let script = "-42";
        let compilation_scope = get_compilation_context(script);
        assert!(!compilation_scope.has_non_static_value);
    }

    #[test]
    fn compile_auto_static_value_detection() {
        let script = "1u8";
        let (res, _) = compile_script_or_return_static_value(
            script,
            CompileOptions::default(),
        )
        .unwrap();
        assert_matches!(
            res,
            StaticValueOrDXB::StaticValue(val) if val == Some(TypedInteger::from(1u8).into())
        );

        let script = "1u8 + 2u8";
        let (res, _) = compile_script_or_return_static_value(
            script,
            CompileOptions::default(),
        )
        .unwrap();
        assert_matches!(
            res,
            StaticValueOrDXB::DXB(code) if code == vec![
                InstructionCode::ADD.into(),
                InstructionCode::UINT_8.into(),
                1,
                InstructionCode::UINT_8.into(),
                2,
            ]
        );
    }

    #[test]
    fn remote_execution() {
        let script = "42u8 :: 43u8";
        let (res, _) =
            compile_script(script, CompileOptions::default()).unwrap();
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
        let (res, _) =
            compile_script(script, CompileOptions::default()).unwrap();
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
    fn remote_execution_injected_const() {
        init_logger_debug();
        let script = "const x = 42u8; 1u8 :: x";
        let (res, _) =
            compile_script(script, CompileOptions::default()).unwrap();
        assert_eq!(
            res,
            vec![
                InstructionCode::SHORT_STATEMENTS.into(),
                2,
                0, // not terminated
                InstructionCode::ALLOCATE_SLOT.into(),
                // slot index as u32
                0,
                0,
                0,
                0,
                InstructionCode::UINT_8.into(),
                42,
                InstructionCode::REMOTE_EXECUTION.into(),
                // --- start of block
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
                // slot 0 (mapped from slot 0)
                InstructionCode::GET_SLOT.into(),
                // slot index as u32
                0,
                0,
                0,
                0,
                // --- end of block
                // caller (literal value 1 for test)
                InstructionCode::UINT_8.into(),
                1,
            ]
        );
    }

    #[test]
    fn remote_execution_injected_var() {
        init_logger_debug();
        // var x only refers to a value, not a ref, but since it is transferred to a
        // remote context, its state is synced via a ref (VariableReference model)
        let script = "var x = 42u8; 1u8 :: x; x = 43u8;";
        let (res, _) =
            compile_script(script, CompileOptions::default()).unwrap();
        assert_eq!(
            res,
            vec![
                InstructionCode::SHORT_STATEMENTS.into(),
                3,
                1, // terminated
                InstructionCode::ALLOCATE_SLOT.into(),
                // slot index as u32
                0,
                0,
                0,
                0,
                InstructionCode::UINT_8.into(),
                42,
                InstructionCode::ALLOCATE_SLOT.into(),
                // slot index as u32
                1,
                0,
                0,
                0,
                // create ref
                InstructionCode::CREATE_REF.into(),
                // slot 0
                InstructionCode::GET_SLOT.into(),
                // slot index as u32
                0,
                0,
                0,
                0,
                InstructionCode::REMOTE_EXECUTION.into(),
                // --- start of block
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
                // slot 0 (mapped from slot 0)
                InstructionCode::GET_SLOT.into(),
                // slot index as u32
                0,
                0,
                0,
                0,
                // --- end of block
                // caller (literal value 1 for test)
                InstructionCode::UINT_8.into(),
                1,
                // TODO #238: this is not the correct slot assignment for VariableReference model
                // set x to 43
                InstructionCode::SET_SLOT.into(),
                // slot index as u32
                0,
                0,
                0,
                0,
                InstructionCode::UINT_8.into(),
                43,
            ]
        );
    }

    #[test]
    fn remote_execution_injected_consts() {
        let script = "const x = 42u8; const y = 69u8; 1u8 :: x + y";
        let (res, _) =
            compile_script(script, CompileOptions::default()).unwrap();
        assert_eq!(
            res,
            vec![
                InstructionCode::SHORT_STATEMENTS.into(),
                3,
                0, // not terminated
                InstructionCode::ALLOCATE_SLOT.into(),
                // slot index as u32
                0,
                0,
                0,
                0,
                InstructionCode::UINT_8.into(),
                42,
                InstructionCode::ALLOCATE_SLOT.into(),
                // slot index as u32
                1,
                0,
                0,
                0,
                InstructionCode::UINT_8.into(),
                69,
                InstructionCode::REMOTE_EXECUTION.into(),
                // --- start of block
                // block size (11 bytes)
                11,
                0,
                0,
                0,
                // injected slots (2)
                2,
                0,
                0,
                0,
                // slot 0
                0,
                0,
                0,
                0,
                // slot 1
                1,
                0,
                0,
                0,
                // expression: x + y
                InstructionCode::ADD.into(),
                InstructionCode::GET_SLOT.into(),
                // slot index as u32
                0,
                0,
                0,
                0,
                InstructionCode::GET_SLOT.into(),
                // slot index as u32
                1,
                0,
                0,
                0,
                // --- end of block
                // caller (literal value 1 for test)
                InstructionCode::UINT_8.into(),
                1,
            ]
        );
    }

    #[test]
    fn remote_execution_shadow_const() {
        let script =
            "const x = 42u8; const y = 69u8; 1u8 :: (const x = 5u8; x + y)";
        let (res, _) =
            compile_script(script, CompileOptions::default()).unwrap();
        assert_eq!(
            res,
            vec![
                InstructionCode::SHORT_STATEMENTS.into(),
                3,
                0, // not terminated
                InstructionCode::ALLOCATE_SLOT.into(),
                // slot index as u32
                0,
                0,
                0,
                0,
                InstructionCode::UINT_8.into(),
                42,
                InstructionCode::ALLOCATE_SLOT.into(),
                // slot index as u32
                1,
                0,
                0,
                0,
                InstructionCode::UINT_8.into(),
                69,
                InstructionCode::REMOTE_EXECUTION.into(),
                // --- start of block
                // block size (21 bytes)
                21,
                0,
                0,
                0,
                // injected slots (1)
                1,
                0,
                0,
                0,
                // slot 1 (y)
                1,
                0,
                0,
                0,
                InstructionCode::SHORT_STATEMENTS.into(),
                2,
                0, // not terminated
                // allocate slot for x
                InstructionCode::ALLOCATE_SLOT.into(),
                // slot index as u32
                1,
                0,
                0,
                0,
                InstructionCode::UINT_8.into(),
                5,
                // expression: x + y
                InstructionCode::ADD.into(),
                InstructionCode::GET_SLOT.into(),
                // slot index as u32
                1,
                0,
                0,
                0,
                InstructionCode::GET_SLOT.into(),
                // slot index as u32
                0,
                0,
                0,
                0,
                // --- end of block
                // caller (literal value 1 for test)
                InstructionCode::UINT_8.into(),
                1,
            ]
        );
    }

    #[test]
    fn remote_execution_nested() {
        let script = "const x = 42u8; (1u8 :: (2u8 :: x))";
        let (res, _) =
            compile_script(script, CompileOptions::default()).unwrap();

        assert_eq!(
            res,
            vec![
                InstructionCode::SHORT_STATEMENTS.into(),
                2,
                0, // not terminated
                InstructionCode::ALLOCATE_SLOT.into(),
                // slot index as u32
                0,
                0,
                0,
                0,
                InstructionCode::UINT_8.into(),
                42,
                InstructionCode::REMOTE_EXECUTION.into(),
                // --- start of block 1
                // block size (20 bytes)
                20,
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
                InstructionCode::GET_SLOT.into(),
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
        let script = "const x = 42u8; (1u8 :: (x :: x))";
        let (res, _) =
            compile_script(script, CompileOptions::default()).unwrap();

        assert_eq!(
            res,
            vec![
                InstructionCode::SHORT_STATEMENTS.into(),
                2,
                0, // not terminated
                InstructionCode::ALLOCATE_SLOT.into(),
                // slot index as u32
                0,
                0,
                0,
                0,
                InstructionCode::UINT_8.into(),
                42,
                InstructionCode::REMOTE_EXECUTION.into(),
                // --- start of block 1
                // block size (23 bytes)
                23,
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
                InstructionCode::GET_SLOT.into(),
                // slot index as u32
                0,
                0,
                0,
                0,
                // --- end of block 2
                // caller (literal value 2 for test)
                InstructionCode::GET_SLOT.into(),
                0,
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
        init_logger_debug();
        let script = "const a = 42; a = 43";
        let result = compile_script(script, CompileOptions::default())
            .map_err(|e| e.error);
        assert_matches!(result, Err(CompilerError::AssignmentToConst { .. }));
    }

    #[test]
    fn assignment_to_const_mut() {
        init_logger_debug();
        let script = "const a = &mut 42; a = 43";
        let result = compile_script(script, CompileOptions::default())
            .map_err(|e| e.error);
        assert_matches!(result, Err(CompilerError::AssignmentToConst { .. }));
    }

    #[test]
    fn internal_assignment_to_const_mut() {
        init_logger_debug();
        let script = "const a = &mut 42; *a = 43";
        let result = compile_script(script, CompileOptions::default());
        assert_matches!(result, Ok(_));
    }

    #[test]
    fn addition_to_const_mut_ref() {
        init_logger_debug();
        let script = "const a = &mut 42; *a += 1;";
        let result = compile_script(script, CompileOptions::default());
        assert_matches!(result, Ok(_));
    }

    #[test]
    fn addition_to_const_variable() {
        init_logger_debug();
        let script = "const a = 42; a += 1";
        let result = compile_script(script, CompileOptions::default())
            .map_err(|e| e.error);
        assert_matches!(result, Err(CompilerError::AssignmentToConst { .. }));
    }

    #[test]
    fn slot_endpoint() {
        let script = "#endpoint";
        let (res, _) =
            compile_script(script, CompileOptions::default()).unwrap();
        assert_eq!(
            res,
            vec![
                InstructionCode::GET_SLOT.into(),
                // slot index as u32
                0,
                0xff,
                0xff,
                0xff
            ]
        );
    }

    // this is not a valid Datex script, just testing the compiler
    #[test]
    fn deref() {
        let script = "*10u8";
        let (res, _) =
            compile_script(script, CompileOptions::default()).unwrap();
        assert_eq!(
            res,
            vec![
                InstructionCode::DEREF.into(),
                InstructionCode::UINT_8.into(),
                // integer as u8
                10,
            ]
        );
    }

    #[test]
    fn type_literal_integer() {
        let script = "type(1)";
        let (res, _) =
            compile_script(script, CompileOptions::default()).unwrap();
        assert_eq!(
            res,
            vec![
                InstructionCode::TYPE_EXPRESSION.into(),
                TypeInstructionCode::TYPE_LITERAL_INTEGER.into(),
                // slot index as u32
                2,
                1,
                0,
                0,
                0,
                1
            ]
        );
    }

    #[test]
    fn type_core_type_integer() {
        let script = "integer";
        let (res, _) =
            compile_script(script, CompileOptions::default()).unwrap();
        let mut instructions: Vec<u8> =
            vec![InstructionCode::GET_INTERNAL_REF.into()];
        // pointer id
        instructions.append(
            &mut PointerAddress::from(CoreLibPointerId::Integer(None))
                .bytes()
                .to_vec(),
        );
        assert_eq!(res, instructions);
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
                InstructionCode::GET_INTERNAL_REF.into(),
                // pointer id for integer
                100,
                0,
                0,
                InstructionCode::UNBOUNDED_STATEMENTS_END.into(),
                0, // unterminated
            ],
        ];

        assert_unbounded_input_matches_output(input, expected_output);
    }
}
