use crate::ast::assignment_operation::AssignmentOperator;
use crate::ast::binding::VariableId;
use crate::compiler::error::CompilerError;
use crate::global::dxb_block::DXBBlock;
use crate::global::protocol_structures::block_header::BlockHeader;
use crate::global::protocol_structures::encrypted_header::EncryptedHeader;
use crate::global::protocol_structures::routing_header::RoutingHeader;

use crate::ast::{DatexScriptParser, parse};
use crate::compiler::context::{CompilationContext, VirtualSlot};
use crate::compiler::metadata::CompileMetadata;
use crate::compiler::precompiler::{
    AstMetadata, AstWithMetadata, VariableMetadata, precompile_ast,
};
use crate::compiler::scope::CompilationScope;
use crate::compiler::type_compiler::compile_type_expression;
use crate::global::instruction_codes::InstructionCode;
use crate::global::slots::InternalSlot;
use crate::libs::core::CoreLibPointerId;
use crate::values::core_values::decimal::Decimal;
use crate::values::pointer::PointerAddress;
use crate::values::value_container::ValueContainer;
use log::info;
use std::cell::RefCell;
use std::rc::Rc;
use crate::ast::parse_result::ValidDatexParseResult;
use crate::ast::tree::{DatexExpression, DatexExpressionData, Slot, Statements, UnaryOperation, VariableAccess, VariableAssignment, VariableDeclaration, VariableKind};

pub mod context;
pub mod error;
pub mod metadata;
pub mod precompiler;
pub mod scope;
mod type_compiler;
mod type_inference;
pub mod workspace;

#[derive(Clone, Default)]
pub struct CompileOptions<'a> {
    pub parser: Option<&'a DatexScriptParser<'a>>,
    pub compile_scope: CompilationScope,
}

impl CompileOptions<'_> {
    pub fn new_with_scope(compile_scope: CompilationScope) -> Self {
        CompileOptions {
            parser: None,
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
        is_end_of_source_text: bool,
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
            || !is_end_of_source_text
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
        is_end_of_source_text: bool,
    ) -> Self {
        let variable_metadata =
            variable_id.and_then(|id| ast_metadata.variable_metadata(id));
        Self::infer(
            variable_kind,
            variable_metadata.cloned(),
            is_end_of_source_text,
        )
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
pub fn compile_block(datex_script: &str) -> Result<Vec<u8>, CompilerError> {
    let (body, _) = compile_script(datex_script, CompileOptions::default())?;

    let routing_header = RoutingHeader::default();

    let block_header = BlockHeader::default();
    let encrypted_header = EncryptedHeader::default();

    let block =
        DXBBlock::new(routing_header, block_header, encrypted_header, body);

    let bytes = block
        .to_bytes()
        .map_err(CompilerError::SerializationError)?;
    Ok(bytes)
}

/// Compiles a DATEX script text into a DXB body
pub fn compile_script<'a>(
    datex_script: &'a str,
    options: CompileOptions<'a>,
) -> Result<(Vec<u8>, CompilationScope), CompilerError> {
    compile_template(datex_script, &[], options)
}

/// Directly extracts a static value from a DATEX script as a `ValueContainer`.
/// This only works if the script does not contain any dynamic values or operations.
/// All JSON-files can be compiled to static values, but not all DATEX scripts.
pub fn extract_static_value_from_script(
    datex_script: &str,
) -> Result<Option<ValueContainer>, CompilerError> {
    let valid_parse_result = parse(datex_script).to_result()?;
    extract_static_value_from_ast(&valid_parse_result.ast).map(Some)
}


/// Converts a DATEX script template text with inserted values into an AST with metadata
/// If the script does not contain any dynamic values or operations, the static result value is
/// directly returned instead of the AST.
pub fn compile_script_or_return_static_value<'a>(
    datex_script: &'a str,
    mut options: CompileOptions<'a>,
) -> Result<(StaticValueOrDXB, CompilationScope), CompilerError> {
    let ast = parse_datex_script_to_ast(
        datex_script,
        &mut options,
    )?;
    let compilation_context = CompilationContext::new(
        RefCell::new(Vec::with_capacity(256)),
        vec![],
        options.compile_scope.once,
    );
    // FIXME: no clone here
    let scope = compile_ast(ast.clone(), &compilation_context, options)?;
    if *compilation_context.has_non_static_value.borrow() {
        Ok((StaticValueOrDXB::DXB(compilation_context.buffer.take()), scope))
    } else {
        // try to extract static value from AST
        extract_static_value_from_ast(ast.ast.as_ref().unwrap())
            .map(|value| (StaticValueOrDXB::StaticValue(Some(value)), scope))
    }
}

/// Parses and precompiles a DATEX script template text with inserted values into an AST with metadata
pub fn parse_datex_script_to_ast<'a>(
    datex_script: &'a str,
    options: &mut CompileOptions<'a>,
) -> Result<AstWithMetadata, CompilerError> {
    // TODO: do this (somewhere else)
    // // shortcut if datex_script is "?" - call compile_value directly
    // if datex_script == "?" {
    //     if inserted_values.len() != 1 {
    //         return Err(CompilerError::InvalidPlaceholderCount);
    //     }
    //     let result =
    //         compile_value(inserted_values[0]).map(StaticValueOrAst::from)?;
    //     return Ok((result, options.compile_scope));
    // }

    let valid_parse_result = parse(datex_script).to_result()?;
    precompile_to_ast_with_metadata(valid_parse_result, &mut options.compile_scope)
}

/// Compiles a DATEX script template text with inserted values into a DXB body
pub fn compile_template<'a>(
    datex_script: &'a str,
    inserted_values: &[ValueContainer],
    mut options: CompileOptions<'a>,
) -> Result<(Vec<u8>, CompilationScope), CompilerError> {
    let ast = parse_datex_script_to_ast(
        datex_script,
        &mut options,
    )?;
    let compilation_context = CompilationContext::new(
        RefCell::new(Vec::with_capacity(256)),
        // TODO: no clone here
        inserted_values.iter().cloned().collect::<Vec<_>>(),
        options.compile_scope.once,
    );
    compile_ast(ast, &compilation_context, options)
        .map(|scope| (compilation_context.buffer.take(), scope))
}

/// Compiles a precompiled DATEX AST, returning the compilation context and scope
fn compile_ast<'a>(
    ast: AstWithMetadata,
    compilation_context: &CompilationContext,
    options: CompileOptions<'a>,
) -> Result<CompilationScope, CompilerError> {
    let compilation_scope = compile_ast_with_metadata(compilation_context, ast, options.compile_scope)?;
    Ok(compilation_scope)
}

pub fn compile_value(value: &ValueContainer) -> Result<Vec<u8>, CompilerError> {
    let buffer = RefCell::new(Vec::with_capacity(256));
    let compilation_scope = CompilationContext::new(buffer, vec![], true);

    compilation_scope.insert_value_container(value);

    Ok(compilation_scope.buffer.take())
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
    ValueContainer::try_from(&ast.data).map_err(|_| CompilerError::NonStaticValue)
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
pub fn precompile_to_ast_with_metadata(
    valid_parse_result: ValidDatexParseResult,
    scope: &mut CompilationScope,
) -> Result<AstWithMetadata, CompilerError> {
    // if once is set to true in already used, return error
    if scope.once {
        if scope.was_used {
            return Err(CompilerError::OnceScopeUsedMultipleTimes);
        }
        // set was_used to true
        scope.was_used = true;
    }
    let ast_with_metadata =
        if let Some(precompiler_data) = &scope.precompiler_data {
            // precompile the AST, adding metadata for variables etc.
            precompile_ast(
                valid_parse_result,
                precompiler_data.ast_with_metadata.metadata.clone(),
                &mut precompiler_data.precompiler_scope_stack.borrow_mut(),
            )?
        } else {
            // if no precompiler data, just use the AST with default metadata
            AstWithMetadata::new_without_metadata(valid_parse_result.ast)
        };

    Ok(ast_with_metadata)
}


pub fn compile_ast_with_metadata(
    compilation_context: &CompilationContext,
    ast_with_metadata: AstWithMetadata,
    scope: CompilationScope,
) -> Result<CompilationScope, CompilerError> {
    let scope = compile_expression(
        compilation_context,
        ast_with_metadata,
        CompileMetadata::outer(),
        scope,
    )?;

    // handle scope virtual addr mapping
    compilation_context.remap_virtual_slots();
    Ok(scope)
}

fn compile_expression(
    compilation_context: &CompilationContext,
    ast_with_metadata: AstWithMetadata,
    meta: CompileMetadata,
    mut scope: CompilationScope,
) -> Result<CompilationScope, CompilerError> {
    let metadata = ast_with_metadata.metadata;
    // TODO: no clone
    match ast_with_metadata.ast.as_ref().unwrap().clone().data {
        DatexExpressionData::Integer(int) => {
            compilation_context
                .insert_encoded_integer(&int.to_smallest_fitting());
        }
        DatexExpressionData::TypedInteger(typed_int) => {
            compilation_context.insert_typed_integer(&typed_int);
        }
        DatexExpressionData::Decimal(decimal) => match &decimal {
            Decimal::Finite(big_decimal) if big_decimal.is_integer() => {
                if let Some(int) = big_decimal.to_i16() {
                    compilation_context.insert_float_as_i16(int);
                } else if let Some(int) = big_decimal.to_i32() {
                    compilation_context.insert_float_as_i32(int);
                } else {
                    compilation_context.insert_decimal(&decimal);
                }
            }
            _ => {
                compilation_context.insert_decimal(&decimal);
            }
        },
        DatexExpressionData::TypedDecimal(typed_decimal) => {
            compilation_context.insert_typed_decimal(&typed_decimal);
        }
        DatexExpressionData::Text(text) => {
            compilation_context.insert_text(&text);
        }
        DatexExpressionData::Boolean(boolean) => {
            compilation_context.insert_boolean(boolean);
        }
        DatexExpressionData::Endpoint(endpoint) => {
            compilation_context.insert_endpoint(&endpoint);
        }
        DatexExpressionData::Null => {
            compilation_context.append_instruction_code(InstructionCode::NULL);
        }
        DatexExpressionData::List(list) => {
            compilation_context
                .append_instruction_code(InstructionCode::LIST_START);
            for item in list {
                scope = compile_expression(
                    compilation_context,
                    AstWithMetadata::new(item, &metadata),
                    CompileMetadata::default(),
                    scope,
                )?;
            }
            compilation_context
                .append_instruction_code(InstructionCode::SCOPE_END);
        }
        DatexExpressionData::Map(map) => {
            // TODO #434: Handle string keyed maps (structs)
            compilation_context
                .append_instruction_code(InstructionCode::MAP_START);
            for (key, value) in map {
                scope = compile_key_value_entry(
                    compilation_context,
                    key,
                    value,
                    &metadata,
                    scope,
                )?;
            }
            compilation_context
                .append_instruction_code(InstructionCode::SCOPE_END);
        }
        DatexExpressionData::Placeholder => {
            compilation_context.insert_value_container(
                compilation_context
                    .inserted_values
                    .borrow()
                    .get(compilation_context.inserted_value_index.get())
                    .unwrap(),
            );
            compilation_context.inserted_value_index.update(|x| x + 1);
        }

        // statements
        DatexExpressionData::Statements(Statements {mut statements, is_terminated}) => {
            compilation_context.mark_has_non_static_value();
            // if single statement and not terminated, just compile the expression
            if statements.len() == 1 && !is_terminated {
                scope = compile_expression(
                    compilation_context,
                    AstWithMetadata::new(
                        statements.remove(0),
                        &metadata,
                    ),
                    CompileMetadata::default(),
                    scope,
                )?;
            } else {
                // if not outer context, new scope
                let mut child_scope = if !meta.is_outer_context() {
                    compilation_context
                        .append_instruction_code(InstructionCode::SCOPE_START);
                    scope.push()
                } else {
                    scope
                };
                let len = statements.len();
                for (i, statement) in statements.into_iter().enumerate() {
                    child_scope = compile_expression(
                        compilation_context,
                        AstWithMetadata::new(statement, &metadata),
                        CompileMetadata::default(),
                        child_scope,
                    )?;
                    // if not last statement or is terminated, append close and store
                    if i < len - 1 || is_terminated {
                        compilation_context.append_instruction_code(
                            InstructionCode::CLOSE_AND_STORE,
                        );
                    }
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
                    compilation_context
                        .append_instruction_code(InstructionCode::SCOPE_END);
                } else {
                    scope = child_scope;
                }
            }
        }

        // unary operations (negation, not, etc.)
        DatexExpressionData::UnaryOperation(UnaryOperation {operator, expression}) => {
            compilation_context
                .append_instruction_code(InstructionCode::from(&operator));
            scope = compile_expression(
                compilation_context,
                AstWithMetadata::new(*expression, &metadata),
                CompileMetadata::default(),
                scope,
            )?;
        }

        // operations (add, subtract, multiply, divide, etc.)
        DatexExpressionData::BinaryOperation(operator, a, b, _) => {
            compilation_context.mark_has_non_static_value();
            // append binary code for operation if not already current binary operator
            compilation_context
                .append_instruction_code(InstructionCode::from(&operator));
            scope = compile_expression(
                compilation_context,
                AstWithMetadata::new(*a, &metadata),
                CompileMetadata::default(),
                scope,
            )?;
            scope = compile_expression(
                compilation_context,
                AstWithMetadata::new(*b, &metadata),
                CompileMetadata::default(),
                scope,
            )?;
        }

        // comparisons (e.g., equal, not equal, greater than, etc.)
        DatexExpressionData::ComparisonOperation(operator, a, b) => {
            compilation_context.mark_has_non_static_value();
            // append binary code for operation if not already current binary operator
            compilation_context
                .append_instruction_code(InstructionCode::from(&operator));
            scope = compile_expression(
                compilation_context,
                AstWithMetadata::new(*a, &metadata),
                CompileMetadata::default(),
                scope,
            )?;
            scope = compile_expression(
                compilation_context,
                AstWithMetadata::new(*b, &metadata),
                CompileMetadata::default(),
                scope,
            )?;
        }

        // apply
        DatexExpressionData::ApplyChain(val, operands) => {
            compilation_context.mark_has_non_static_value();
            // TODO #150
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
                AstWithMetadata::new(*value, &metadata),
                CompileMetadata::default(),
                scope,
            )?;

            let variable_model =
                VariableModel::infer_from_ast_metadata_and_type(
                    &metadata.borrow(),
                    id,
                    kind,
                    compilation_context.is_end_of_source_text,
                );
            info!("variable model for {name}: {variable_model:?}");

            // create new variable depending on the model
            let variable = match variable_model {
                VariableModel::VariableReference => {
                    // scope end
                    compilation_context
                        .append_instruction_code(InstructionCode::SCOPE_END);
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

            compilation_context
                .append_instruction_code(InstructionCode::SCOPE_END);
        }

        DatexExpressionData::GetReference(address) => {
            compilation_context.mark_has_non_static_value();
            compilation_context.insert_get_ref(address);
        }

        // assignment
        DatexExpressionData::VariableAssignment(VariableAssignment { 
            operator,
            name, 
            expression, .. 
        }) => {
            compilation_context.mark_has_non_static_value();
            // get variable slot address
            let (virtual_slot, kind) = scope
                .resolve_variable_name_to_virtual_slot(&name)
                .ok_or_else(|| {
                    CompilerError::UndeclaredVariable(name.clone())
                })?;

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
                op => todo!("#436 Handle assignment operator: {op:?}"),
            }

            compilation_context.insert_virtual_slot_address(virtual_slot);
            // compile expression
            scope = compile_expression(
                compilation_context,
                AstWithMetadata::new(*expression, &metadata),
                CompileMetadata::default(),
                scope,
            )?;
            // close assignment scope
            compilation_context
                .append_instruction_code(InstructionCode::SCOPE_END);
        }

        DatexExpressionData::DerefAssignment {
            operator,
            deref_count,
            deref_expression,
            assigned_expression,
        } => {
            compilation_context.mark_has_non_static_value();

            compilation_context
                .append_instruction_code(InstructionCode::ASSIGN_TO_REF);

            compilation_context
                .append_instruction_code(InstructionCode::from(&operator));

            // "*x" must not be dereferenced, x is already the relevant reference that is modified
            for _ in 0..deref_count - 1 {
                compilation_context
                    .append_instruction_code(InstructionCode::DEREF);
            }

            // compile deref expression
            scope = compile_expression(
                compilation_context,
                AstWithMetadata::new(*deref_expression, &metadata),
                CompileMetadata::default(),
                scope,
            )?;

            for _ in 0..deref_count - 1 {
                compilation_context
                    .append_instruction_code(InstructionCode::SCOPE_END);
            }

            // compile assigned expression
            scope = compile_expression(
                compilation_context,
                AstWithMetadata::new(*assigned_expression, &metadata),
                CompileMetadata::default(),
                scope,
            )?;

            // close assignment scope
            compilation_context
                .append_instruction_code(InstructionCode::SCOPE_END);
        }

        // variable access
        DatexExpressionData::VariableAccess(VariableAccess { name, .. }) => {
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
        DatexExpressionData::RemoteExecution(caller, script) => {
            compilation_context.mark_has_non_static_value();

            // insert remote execution code
            compilation_context
                .append_instruction_code(InstructionCode::REMOTE_EXECUTION);
            // insert compiled caller expression
            scope = compile_expression(
                compilation_context,
                AstWithMetadata::new(*caller, &metadata),
                CompileMetadata::default(),
                scope,
            )?;

            // compile remote execution block
            let execution_block_ctx = CompilationContext::new(
                RefCell::new(Vec::with_capacity(256)),
                vec![],
                true,
            );
            let external_scope = compile_ast_with_metadata(
                &execution_block_ctx,
                AstWithMetadata::new(*script, &metadata),
                CompilationScope::new_with_external_parent_scope(scope),
            )?;
            // reset to current scope
            scope = external_scope
                .pop_external()
                .ok_or_else(|| CompilerError::ScopePopError)?;

            let external_slots = execution_block_ctx.external_slots();
            // start block
            compilation_context
                .append_instruction_code(InstructionCode::EXECUTION_BLOCK);
            // set block size (len of compilation_context.buffer)
            compilation_context
                .append_u32(execution_block_ctx.buffer.borrow().len() as u32);
            // set injected slot count
            compilation_context.append_u32(external_slots.len() as u32);
            for slot in external_slots {
                compilation_context.insert_virtual_slot_address(slot.upgrade());
            }

            // insert block body (compilation_context.buffer)
            compilation_context
                .append_buffer(&execution_block_ctx.buffer.borrow())
        }

        // named slot
        DatexExpressionData::Slot(Slot::Named(name)) => {
            match name.as_str() {
                "endpoint" => {
                    compilation_context
                        .append_instruction_code(InstructionCode::GET_SLOT);
                    compilation_context
                        .append_u32(InternalSlot::ENDPOINT as u32);
                }
                "core" => compilation_context.insert_get_ref(
                    PointerAddress::from(CoreLibPointerId::Core),
                ),
                _ => {
                    // invalid slot name
                    return Err(CompilerError::InvalidSlotName(name.clone()));
                }
            }
        }

        // pointer address
        DatexExpressionData::PointerAddress(address) => {
            compilation_context.insert_get_ref(address);
        }

        // refs
        DatexExpressionData::CreateRef(expression) => {
            compilation_context.mark_has_non_static_value();
            compilation_context
                .append_instruction_code(InstructionCode::CREATE_REF);
            scope = compile_expression(
                compilation_context,
                AstWithMetadata::new(*expression, &metadata),
                CompileMetadata::default(),
                scope,
            )?;
        }
        DatexExpressionData::CreateRefMut(expression) => {
            compilation_context.mark_has_non_static_value();
            compilation_context
                .append_instruction_code(InstructionCode::CREATE_REF_MUT);
            scope = compile_expression(
                compilation_context,
                AstWithMetadata::new(*expression, &metadata),
                CompileMetadata::default(),
                scope,
            )?;
        }
        DatexExpressionData::CreateRefFinal(expression) => {
            compilation_context.mark_has_non_static_value();
            compilation_context
                .append_instruction_code(InstructionCode::CREATE_REF_FINAL);
            scope = compile_expression(
                compilation_context,
                AstWithMetadata::new(*expression, &metadata),
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

        DatexExpressionData::Deref(expression) => {
            compilation_context.mark_has_non_static_value();
            compilation_context.append_instruction_code(InstructionCode::DEREF);
            scope = compile_expression(
                compilation_context,
                AstWithMetadata::new(*expression, &metadata),
                CompileMetadata::default(),
                scope,
            )?;
            compilation_context
                .append_instruction_code(InstructionCode::SCOPE_END);
        }

        _ => {
            return Err(CompilerError::UnexpectedTerm(Box::new(
                ast_with_metadata.ast.unwrap(),
            )));
        }
    }

    Ok(scope)
}

fn compile_key_value_entry(
    compilation_scope: &CompilationContext,
    key: DatexExpression,
    value: DatexExpression,
    metadata: &Rc<RefCell<AstMetadata>>,
    mut scope: CompilationScope,
) -> Result<CompilationScope, CompilerError> {
    match key.data {
        // text -> insert key string
        DatexExpressionData::Text(text) => {
            compilation_scope.insert_key_string(&text);
        }
        // other -> insert key as dynamic
        _ => {
            compilation_scope
                .append_instruction_code(InstructionCode::KEY_VALUE_DYNAMIC);
            scope = compile_expression(
                compilation_scope,
                AstWithMetadata::new(key, metadata),
                CompileMetadata::default(),
                scope,
            )?;
        }
    };
    // insert value
    scope = compile_expression(
        compilation_scope,
        AstWithMetadata::new(value, metadata),
        CompileMetadata::default(),
        scope,
    )?;
    Ok(scope)
}

#[cfg(test)]
pub mod tests {
    use super::{CompilationContext, CompilationScope, CompileOptions, StaticValueOrDXB, compile_ast, compile_script, compile_script_or_return_static_value, compile_template, parse_datex_script_to_ast};
    use std::assert_matches::assert_matches;
    use std::cell::RefCell;
    use std::io::Read;
    use std::vec;

    use crate::ast::parse;
    use crate::global::type_instruction_codes::TypeSpaceInstructionCode;
    use crate::libs::core::CoreLibPointerId;
    use crate::values::core_values::integer::Integer;
    use crate::values::pointer::PointerAddress;
    use crate::{
        global::instruction_codes::InstructionCode, logger::init_logger_debug,
    };
    use datex_core::compiler::error::CompilerError;
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
        let ast = parse_datex_script_to_ast(script, &mut options).unwrap();

        let compilation_context = CompilationContext::new(
            RefCell::new(Vec::with_capacity(256)),
            vec![],
            options.compile_scope.once,
        );
        compile_ast(ast, &compilation_context, options)
            .unwrap();
        compilation_context
    }

    #[test]
    fn simple_multiplication() {
        init_logger_debug();

        let lhs: u8 = 1;
        let rhs: u8 = 2;
        let datex_script = format!("{lhs} * {rhs}"); // 1 * 2
        let result = compile_and_log(&datex_script);
        assert_eq!(
            result,
            vec![
                InstructionCode::MULTIPLY.into(),
                InstructionCode::INT_8.into(),
                lhs,
                InstructionCode::INT_8.into(),
                rhs,
            ]
        );
    }

    #[test]
    fn simple_multiplication_close() {
        init_logger_debug();

        let lhs: u8 = 1;
        let rhs: u8 = 2;
        let datex_script = format!("{lhs} * {rhs};"); // 1 * 2
        let result = compile_and_log(&datex_script);
        assert_eq!(
            result,
            vec![
                InstructionCode::MULTIPLY.into(),
                InstructionCode::INT_8.into(),
                lhs,
                InstructionCode::INT_8.into(),
                rhs,
                InstructionCode::CLOSE_AND_STORE.into()
            ]
        );
    }

    #[test]
    fn is_operator() {
        init_logger_debug();

        // TODO #151: compare refs
        let datex_script = "1 is 2".to_string();
        let result = compile_and_log(&datex_script);
        assert_eq!(
            result,
            vec![
                InstructionCode::IS.into(),
                InstructionCode::INT_8.into(),
                1,
                InstructionCode::INT_8.into(),
                2
            ]
        );

        let datex_script =
            "const a = &mut 42; const b = &mut 69; a is b".to_string(); // a is b
        let result = compile_and_log(&datex_script);
        assert_eq!(
            result,
            vec![
                // val a = 42;
                InstructionCode::ALLOCATE_SLOT.into(),
                0,
                0,
                0,
                0,
                InstructionCode::CREATE_REF_MUT.into(),
                InstructionCode::INT_8.into(),
                42,
                InstructionCode::SCOPE_END.into(),
                InstructionCode::CLOSE_AND_STORE.into(),
                // val b = 69;
                InstructionCode::ALLOCATE_SLOT.into(),
                1,
                0,
                0,
                0,
                InstructionCode::CREATE_REF_MUT.into(),
                InstructionCode::INT_8.into(),
                69,
                InstructionCode::SCOPE_END.into(),
                InstructionCode::CLOSE_AND_STORE.into(),
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
        let datex_script = format!("{lhs} == {rhs}"); // 1 == 2
        let result = compile_and_log(&datex_script);
        assert_eq!(
            result,
            vec![
                InstructionCode::STRUCTURAL_EQUAL.into(),
                InstructionCode::INT_8.into(),
                lhs,
                InstructionCode::INT_8.into(),
                rhs,
            ]
        );

        let datex_script = format!("{lhs} === {rhs}"); // 1 === 2
        let result = compile_and_log(&datex_script);
        assert_eq!(
            result,
            vec![
                InstructionCode::EQUAL.into(),
                InstructionCode::INT_8.into(),
                lhs,
                InstructionCode::INT_8.into(),
                rhs,
            ]
        );

        let datex_script = format!("{lhs} != {rhs}"); // 1 != 2
        let result = compile_and_log(&datex_script);
        assert_eq!(
            result,
            vec![
                InstructionCode::NOT_STRUCTURAL_EQUAL.into(),
                InstructionCode::INT_8.into(),
                lhs,
                InstructionCode::INT_8.into(),
                rhs,
            ]
        );
        let datex_script = format!("{lhs} !== {rhs}"); // 1 !== 2
        let result = compile_and_log(&datex_script);
        assert_eq!(
            result,
            vec![
                InstructionCode::NOT_EQUAL.into(),
                InstructionCode::INT_8.into(),
                lhs,
                InstructionCode::INT_8.into(),
                rhs,
            ]
        );
    }

    #[test]
    fn simple_addition() {
        init_logger_debug();

        let lhs: u8 = 1;
        let rhs: u8 = 2;
        let datex_script = format!("{lhs} + {rhs}"); // 1 + 2
        let result = compile_and_log(&datex_script);
        assert_eq!(
            result,
            vec![
                InstructionCode::ADD.into(),
                InstructionCode::INT_8.into(),
                lhs,
                InstructionCode::INT_8.into(),
                rhs
            ]
        );

        let datex_script = format!("{lhs} + {rhs};"); // 1 + 2;
        let result = compile_and_log(&datex_script);
        assert_eq!(
            result,
            vec![
                InstructionCode::ADD.into(),
                InstructionCode::INT_8.into(),
                lhs,
                InstructionCode::INT_8.into(),
                rhs,
                InstructionCode::CLOSE_AND_STORE.into()
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

        let datex_script = format!("{op1} + {op2} + {op3} + {op4}"); // 1 + 2 + 3 + 4
        let result = compile_and_log(&datex_script);
        assert_eq!(
            result,
            vec![
                InstructionCode::ADD.into(),
                InstructionCode::ADD.into(),
                InstructionCode::ADD.into(),
                InstructionCode::INT_8.into(),
                op1,
                InstructionCode::INT_8.into(),
                op2,
                InstructionCode::INT_8.into(),
                op3,
                InstructionCode::INT_8.into(),
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

        let datex_script = format!("{op1} * {op2} + {op3} * {op4}"); // 1 + 2 + 3 + 4
        let result = compile_and_log(&datex_script);
        assert_eq!(
            result,
            vec![
                InstructionCode::ADD.into(),
                InstructionCode::MULTIPLY.into(),
                InstructionCode::INT_8.into(),
                op1,
                InstructionCode::INT_8.into(),
                op2,
                InstructionCode::MULTIPLY.into(),
                InstructionCode::INT_8.into(),
                op3,
                InstructionCode::INT_8.into(),
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
        let datex_script = format!("{a} + ({b} + {c})"); // 1 + (2 + 3)
        let result = compile_and_log(&datex_script);

        // note: scope is automatically collapsed by the parser since this is all the same operation
        // TODO #152: we might need to change this to support nested additions, or maybe not if we only allow additions
        // of values of the same type?...
        assert_eq!(
            result,
            vec![
                InstructionCode::ADD.into(),
                InstructionCode::INT_8.into(),
                a,
                InstructionCode::ADD.into(),
                InstructionCode::INT_8.into(),
                b,
                InstructionCode::INT_8.into(),
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
        let datex_script = format!("{a} + ({b} - {c})"); // 1 + (2 - 3)
        let result = compile_and_log(&datex_script);
        assert_eq!(
            result,
            vec![
                InstructionCode::ADD.into(),
                InstructionCode::INT_8.into(),
                a,
                InstructionCode::SUBTRACT.into(),
                InstructionCode::INT_8.into(),
                b,
                InstructionCode::INT_8.into(),
                c,
            ]
        );
    }

    #[test]
    fn integer_u8() {
        init_logger_debug();
        let val = 42;
        let datex_script = format!("{val}"); // 42
        let result = compile_and_log(&datex_script);
        assert_eq!(result, vec![InstructionCode::INT_8.into(), val,]);
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
            InstructionCode::LIST_START.into(),
            InstructionCode::SCOPE_END.into(),
        ];
        assert_eq!(result, expected);
    }

    // Test list with single element
    #[test]
    fn single_element_list() {
        init_logger_debug();
        // TODO #438: support list constructor (apply on type)
        let datex_script = "[42]";
        let result = compile_and_log(datex_script);
        assert_eq!(
            result,
            vec![
                InstructionCode::LIST_START.into(),
                InstructionCode::INT_8.into(),
                42,
                InstructionCode::SCOPE_END.into(),
            ]
        );
    }

    // Test list with multiple elements
    #[test]
    fn multi_element_list() {
        init_logger_debug();
        let datex_script = "[1, 2, 3]";
        let result = compile_and_log(datex_script);
        assert_eq!(
            result,
            vec![
                InstructionCode::LIST_START.into(),
                InstructionCode::INT_8.into(),
                1,
                InstructionCode::INT_8.into(),
                2,
                InstructionCode::INT_8.into(),
                3,
                InstructionCode::SCOPE_END.into(),
            ]
        );

        // trailing comma
        let datex_script = "[1, 2, 3,]";
        let result = compile_and_log(datex_script);
        assert_eq!(
            result,
            vec![
                InstructionCode::LIST_START.into(),
                InstructionCode::INT_8.into(),
                1,
                InstructionCode::INT_8.into(),
                2,
                InstructionCode::INT_8.into(),
                3,
                InstructionCode::SCOPE_END.into(),
            ]
        );
    }

    // Test list with expressions inside
    #[test]
    fn list_with_expressions() {
        init_logger_debug();
        let datex_script = "[1 + 2, 3 * 4]";
        let result = compile_and_log(datex_script);
        assert_eq!(
            result,
            vec![
                InstructionCode::LIST_START.into(),
                InstructionCode::ADD.into(),
                InstructionCode::INT_8.into(),
                1,
                InstructionCode::INT_8.into(),
                2,
                InstructionCode::MULTIPLY.into(),
                InstructionCode::INT_8.into(),
                3,
                InstructionCode::INT_8.into(),
                4,
                InstructionCode::SCOPE_END.into(),
            ]
        );
    }

    // Nested lists
    #[test]
    fn nested_lists() {
        init_logger_debug();
        let datex_script = "[1, [2, 3], 4]";
        let result = compile_and_log(datex_script);
        assert_eq!(
            result,
            vec![
                InstructionCode::LIST_START.into(),
                InstructionCode::INT_8.into(),
                1,
                InstructionCode::LIST_START.into(),
                InstructionCode::INT_8.into(),
                2,
                InstructionCode::INT_8.into(),
                3,
                InstructionCode::SCOPE_END.into(),
                InstructionCode::INT_8.into(),
                4,
                InstructionCode::SCOPE_END.into(),
            ]
        );
    }

    // map with text key
    #[test]
    fn map_with_text_key() {
        init_logger_debug();
        let datex_script = "{\"key\": 42}";
        let result = compile_and_log(datex_script);
        let expected = vec![
            InstructionCode::MAP_START.into(),
            InstructionCode::KEY_VALUE_SHORT_TEXT.into(),
            3, // length of "key"
            b'k',
            b'e',
            b'y',
            InstructionCode::INT_8.into(),
            42,
            InstructionCode::SCOPE_END.into(),
        ];
        assert_eq!(result, expected);
    }

    // map with integer key
    #[test]
    fn map_integer_key() {
        init_logger_debug();
        let datex_script = "{(10): 42}";
        let result = compile_and_log(datex_script);
        let expected = vec![
            InstructionCode::MAP_START.into(),
            InstructionCode::KEY_VALUE_DYNAMIC.into(),
            InstructionCode::INT_8.into(),
            10,
            InstructionCode::INT_8.into(),
            42,
            InstructionCode::SCOPE_END.into(),
        ];
        assert_eq!(result, expected);
    }

    // map with long text key (>255 bytes)
    #[test]
    fn map_with_long_text_key() {
        init_logger_debug();
        let long_key = "a".repeat(300);
        let datex_script = format!("{{\"{long_key}\": 42}}");
        let result = compile_and_log(&datex_script);
        let mut expected: Vec<u8> = vec![
            InstructionCode::MAP_START.into(),
            InstructionCode::KEY_VALUE_DYNAMIC.into(),
            InstructionCode::TEXT.into(),
        ];
        expected.extend((long_key.len() as u32).to_le_bytes());
        expected.extend(long_key.as_bytes());
        expected.extend(vec![
            InstructionCode::INT_8.into(),
            42,
            InstructionCode::SCOPE_END.into(),
        ]);
        assert_eq!(result, expected);
    }

    // map with dynamic key (expression)
    #[test]
    fn map_with_dynamic_key() {
        init_logger_debug();
        let datex_script = "{(1 + 2): 42}";
        let result = compile_and_log(datex_script);
        let expected = [
            InstructionCode::MAP_START.into(),
            InstructionCode::KEY_VALUE_DYNAMIC.into(),
            InstructionCode::ADD.into(),
            InstructionCode::INT_8.into(),
            1,
            InstructionCode::INT_8.into(),
            2,
            InstructionCode::INT_8.into(),
            42,
            InstructionCode::SCOPE_END.into(),
        ];
        assert_eq!(result, expected);
    }

    // map with multiple keys (text, integer, expression)
    #[test]
    fn map_with_multiple_keys() {
        init_logger_debug();
        let datex_script = "{key: 42, (4): 43, (1 + 2): 44}";
        let result = compile_and_log(datex_script);
        let expected = vec![
            InstructionCode::MAP_START.into(),
            InstructionCode::KEY_VALUE_SHORT_TEXT.into(),
            3, // length of "key"
            b'k',
            b'e',
            b'y',
            InstructionCode::INT_8.into(),
            42,
            InstructionCode::KEY_VALUE_DYNAMIC.into(),
            InstructionCode::INT_8.into(),
            4,
            InstructionCode::INT_8.into(),
            43,
            InstructionCode::KEY_VALUE_DYNAMIC.into(),
            InstructionCode::ADD.into(),
            InstructionCode::INT_8.into(),
            1,
            InstructionCode::INT_8.into(),
            2,
            InstructionCode::INT_8.into(),
            44,
            InstructionCode::SCOPE_END.into(),
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
            InstructionCode::MAP_START.into(),
            InstructionCode::SCOPE_END.into(),
        ];
        assert_eq!(result, expected);
    }

    #[test]
    fn allocate_slot() {
        init_logger_debug();
        let script = "const a = 42";
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
                InstructionCode::INT_8.into(),
                42,
                InstructionCode::SCOPE_END.into(),
            ]
        );
    }

    #[test]
    fn allocate_slot_with_value() {
        init_logger_debug();
        let script = "const a = 42; a + 1";
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
                InstructionCode::INT_8.into(),
                42,
                InstructionCode::SCOPE_END.into(),
                InstructionCode::CLOSE_AND_STORE.into(),
                InstructionCode::ADD.into(),
                InstructionCode::GET_SLOT.into(),
                // slot index as u32
                0,
                0,
                0,
                0,
                InstructionCode::INT_8.into(),
                1,
            ]
        );
    }

    #[test]
    fn allocate_scoped_slots() {
        init_logger_debug();
        let script = "const a = 42; (const a = 43; a); a";
        let result = compile_and_log(script);
        assert_eq!(
            result,
            vec![
                InstructionCode::ALLOCATE_SLOT.into(),
                0,
                0,
                0,
                0,
                InstructionCode::INT_8.into(),
                42,
                InstructionCode::SCOPE_END.into(),
                InstructionCode::CLOSE_AND_STORE.into(),
                InstructionCode::SCOPE_START.into(),
                InstructionCode::ALLOCATE_SLOT.into(),
                1,
                0,
                0,
                0,
                InstructionCode::INT_8.into(),
                43,
                InstructionCode::SCOPE_END.into(),
                InstructionCode::CLOSE_AND_STORE.into(),
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
                InstructionCode::SCOPE_END.into(),
                InstructionCode::CLOSE_AND_STORE.into(),
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
        let script = "const a = 42; const b = 41; (const a = 43; a; b); a";
        let result = compile_and_log(script);
        assert_eq!(
            result,
            vec![
                InstructionCode::ALLOCATE_SLOT.into(),
                0,
                0,
                0,
                0,
                InstructionCode::INT_8.into(),
                42,
                InstructionCode::SCOPE_END.into(),
                InstructionCode::CLOSE_AND_STORE.into(),
                InstructionCode::ALLOCATE_SLOT.into(),
                1,
                0,
                0,
                0,
                InstructionCode::INT_8.into(),
                41,
                InstructionCode::SCOPE_END.into(),
                InstructionCode::CLOSE_AND_STORE.into(),
                InstructionCode::SCOPE_START.into(),
                InstructionCode::ALLOCATE_SLOT.into(),
                2,
                0,
                0,
                0,
                InstructionCode::INT_8.into(),
                43,
                InstructionCode::SCOPE_END.into(),
                InstructionCode::CLOSE_AND_STORE.into(),
                InstructionCode::GET_SLOT.into(),
                2,
                0,
                0,
                0,
                InstructionCode::CLOSE_AND_STORE.into(),
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
                InstructionCode::SCOPE_END.into(),
                InstructionCode::CLOSE_AND_STORE.into(),
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
        let script = "const a = &mut 42";
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
                InstructionCode::INT_8.into(),
                42,
                InstructionCode::SCOPE_END.into(),
            ]
        );
    }

    #[test]
    fn read_ref() {
        init_logger_debug();
        let script = "const a = &mut 42; a";
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
                InstructionCode::INT_8.into(),
                42,
                InstructionCode::SCOPE_END.into(),
                InstructionCode::CLOSE_AND_STORE.into(),
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
            &[Integer::from(1).into(), Integer::from(2).into()],
            CompileOptions::default(),
        );
        assert_eq!(
            result.unwrap().0,
            vec![
                InstructionCode::ADD.into(),
                InstructionCode::INT_8.into(),
                1,
                InstructionCode::INT_8.into(),
                2
            ]
        );
    }

    #[test]
    fn compile_macro() {
        init_logger_debug();
        let a = Integer::from(1);
        let result = compile!("?", a);
        assert_eq!(result.unwrap().0, vec![InstructionCode::INT_8.into(), 1,]);
    }

    #[test]
    fn compile_macro_multi() {
        init_logger_debug();
        let result = compile!("? + ?", Integer::from(1), Integer::from(2));
        assert_eq!(
            result.unwrap().0,
            vec![
                InstructionCode::ADD.into(),
                InstructionCode::INT_8.into(),
                1,
                InstructionCode::INT_8.into(),
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
        let mut reader = std::io::BufReader::new(file);
        let mut json_string = String::new();
        reader
            .read_to_string(&mut json_string)
            .expect("Failed to read test.json");
        json_string
    }

    #[test]
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
        assert!(*compilation_scope.has_non_static_value.borrow());

        let script = "1 2";
        let compilation_scope = get_compilation_context(script);
        assert!(*compilation_scope.has_non_static_value.borrow());

        let script = "1;2";
        let compilation_scope = get_compilation_context(script);
        assert!(*compilation_scope.has_non_static_value.borrow());

        let script = r#"{("x" + "y"): 1}"#;
        let compilation_scope = get_compilation_context(script);
        assert!(*compilation_scope.has_non_static_value.borrow());

        // static
        let script = "1";
        let compilation_scope = get_compilation_context(script);
        assert!(!*compilation_scope.has_non_static_value.borrow());

        let script = "[]";
        let compilation_scope = get_compilation_context(script);
        assert!(!*compilation_scope.has_non_static_value.borrow());

        let script = "{}";
        let compilation_scope = get_compilation_context(script);
        assert!(!*compilation_scope.has_non_static_value.borrow());

        let script = "[1,2,3]";
        let compilation_scope = get_compilation_context(script);
        assert!(!*compilation_scope.has_non_static_value.borrow());

        let script = "{a: 2}";
        let compilation_scope = get_compilation_context(script);
        assert!(!*compilation_scope.has_non_static_value.borrow());

        // because of unary - 42
        let script = "-42";
        let compilation_scope = get_compilation_context(script);
        assert!(!*compilation_scope.has_non_static_value.borrow());
    }

    #[test]
    fn compile_auto_static_value_detection() {
        let script = "1";
        let (res, _) = compile_script_or_return_static_value(
            script,
            CompileOptions::default(),
        )
        .unwrap();
        assert_matches!(
            res,
            StaticValueOrDXB::StaticValue(val) if val == Some(Integer::from(1).into())
        );

        let script = "1 + 2";
        let (res, _) = compile_script_or_return_static_value(
            script,
            CompileOptions::default(),
        )
        .unwrap();
        assert_matches!(
            res,
            StaticValueOrDXB::DXB(code) if code == vec![
                InstructionCode::ADD.into(),
                InstructionCode::INT_8.into(),
                1,
                InstructionCode::INT_8.into(),
                2,
            ]
        );
    }

    #[test]
    fn remote_execution() {
        let script = "42 :: 43";
        let (res, _) =
            compile_script(script, CompileOptions::default()).unwrap();
        assert_eq!(
            res,
            vec![
                InstructionCode::REMOTE_EXECUTION.into(),
                // caller (literal value 42 for test)
                InstructionCode::INT_8.into(),
                42,
                // start of block
                InstructionCode::EXECUTION_BLOCK.into(),
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
                InstructionCode::INT_8.into(),
                43,
            ]
        );
    }

    #[test]
    fn remote_execution_expression() {
        let script = "42 :: 1 + 2";
        let (res, _) =
            compile_script(script, CompileOptions::default()).unwrap();
        assert_eq!(
            res,
            vec![
                InstructionCode::REMOTE_EXECUTION.into(),
                // caller (literal value 42 for test)
                InstructionCode::INT_8.into(),
                42,
                // start of block
                InstructionCode::EXECUTION_BLOCK.into(),
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
                InstructionCode::INT_8.into(),
                1,
                InstructionCode::INT_8.into(),
                2,
            ]
        );
    }

    #[test]
    fn remote_execution_injected_const() {
        init_logger_debug();
        let script = "const x = 42; 1 :: x";
        let (res, _) =
            compile_script(script, CompileOptions::default()).unwrap();
        assert_eq!(
            res,
            vec![
                InstructionCode::ALLOCATE_SLOT.into(),
                // slot index as u32
                0,
                0,
                0,
                0,
                InstructionCode::INT_8.into(),
                42,
                InstructionCode::SCOPE_END.into(),
                InstructionCode::CLOSE_AND_STORE.into(),
                InstructionCode::REMOTE_EXECUTION.into(),
                // caller (literal value 1 for test)
                InstructionCode::INT_8.into(),
                1,
                // start of block
                InstructionCode::EXECUTION_BLOCK.into(),
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
            ]
        );
    }

    #[test]
    fn remote_execution_injected_var() {
        init_logger_debug();
        // var x only refers to a value, not a ref, but since it is transferred to a
        // remote context, its state is synced via a ref (VariableReference model)
        let script = "var x = 42; 1 :: x; x = 43;";
        let (res, _) =
            compile_script(script, CompileOptions::default()).unwrap();
        assert_eq!(
            res,
            vec![
                InstructionCode::ALLOCATE_SLOT.into(),
                // slot index as u32
                0,
                0,
                0,
                0,
                InstructionCode::INT_8.into(),
                42,
                InstructionCode::SCOPE_END.into(),
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
                InstructionCode::SCOPE_END.into(),
                InstructionCode::CLOSE_AND_STORE.into(),
                InstructionCode::REMOTE_EXECUTION.into(),
                // caller (literal value 1 for test)
                InstructionCode::INT_8.into(),
                1,
                // start of block
                InstructionCode::EXECUTION_BLOCK.into(),
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
                InstructionCode::CLOSE_AND_STORE.into(),
                // TODO #238: this is not the correct slot assignment for VariableReference model
                // set x to 43
                InstructionCode::SET_SLOT.into(),
                // slot index as u32
                0,
                0,
                0,
                0,
                InstructionCode::INT_8.into(),
                43,
                InstructionCode::SCOPE_END.into(),
                InstructionCode::CLOSE_AND_STORE.into(),
            ]
        );
    }

    #[test]
    fn remote_execution_injected_consts() {
        let script = "const x = 42; const y = 69; 1 :: x + y";
        let (res, _) =
            compile_script(script, CompileOptions::default()).unwrap();
        assert_eq!(
            res,
            vec![
                InstructionCode::ALLOCATE_SLOT.into(),
                // slot index as u32
                0,
                0,
                0,
                0,
                InstructionCode::INT_8.into(),
                42,
                InstructionCode::SCOPE_END.into(),
                InstructionCode::CLOSE_AND_STORE.into(),
                InstructionCode::ALLOCATE_SLOT.into(),
                // slot index as u32
                1,
                0,
                0,
                0,
                InstructionCode::INT_8.into(),
                69,
                InstructionCode::SCOPE_END.into(),
                InstructionCode::CLOSE_AND_STORE.into(),
                InstructionCode::REMOTE_EXECUTION.into(),
                // caller (literal value 1 for test)
                InstructionCode::INT_8.into(),
                1,
                // start of block
                InstructionCode::EXECUTION_BLOCK.into(),
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
            ]
        );
    }

    #[test]
    fn remote_execution_shadow_const() {
        let script = "const x = 42; const y = 69; 1 :: (const x = 5; x + y)";
        let (res, _) =
            compile_script(script, CompileOptions::default()).unwrap();
        assert_eq!(
            res,
            vec![
                InstructionCode::ALLOCATE_SLOT.into(),
                // slot index as u32
                0,
                0,
                0,
                0,
                InstructionCode::INT_8.into(),
                42,
                InstructionCode::SCOPE_END.into(),
                InstructionCode::CLOSE_AND_STORE.into(),
                InstructionCode::ALLOCATE_SLOT.into(),
                // slot index as u32
                1,
                0,
                0,
                0,
                InstructionCode::INT_8.into(),
                69,
                InstructionCode::SCOPE_END.into(),
                InstructionCode::CLOSE_AND_STORE.into(),
                InstructionCode::REMOTE_EXECUTION.into(),
                // caller (literal value 1 for test)
                InstructionCode::INT_8.into(),
                1,
                // start of block
                InstructionCode::EXECUTION_BLOCK.into(),
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
                // slot 1 (y)
                1,
                0,
                0,
                0,
                // allocate slot for x
                InstructionCode::ALLOCATE_SLOT.into(),
                // slot index as u32
                1,
                0,
                0,
                0,
                InstructionCode::INT_8.into(),
                5,
                InstructionCode::SCOPE_END.into(),
                InstructionCode::CLOSE_AND_STORE.into(),
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
            ]
        );
    }

    #[test]
    fn remote_execution_nested() {
        let script = "const x = 42; (1 :: (2 :: x))";
        let (res, _) =
            compile_script(script, CompileOptions::default()).unwrap();

        assert_eq!(
            res,
            vec![
                InstructionCode::ALLOCATE_SLOT.into(),
                // slot index as u32
                0,
                0,
                0,
                0,
                InstructionCode::INT_8.into(),
                42,
                InstructionCode::SCOPE_END.into(),
                InstructionCode::CLOSE_AND_STORE.into(),
                InstructionCode::REMOTE_EXECUTION.into(),
                // caller (literal value 1 for test)
                InstructionCode::INT_8.into(),
                1,
                // start of block
                InstructionCode::EXECUTION_BLOCK.into(),
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
                // slot 0
                0,
                0,
                0,
                0,
                // nested remote execution
                InstructionCode::REMOTE_EXECUTION.into(),
                // caller (literal value 2 for test)
                InstructionCode::INT_8.into(),
                2,
                // start of block
                InstructionCode::EXECUTION_BLOCK.into(),
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
            ]
        );
    }

    #[test]
    fn remote_execution_nested2() {
        let script = "const x = 42; (1 :: (x :: x))";
        let (res, _) =
            compile_script(script, CompileOptions::default()).unwrap();

        assert_eq!(
            res,
            vec![
                InstructionCode::ALLOCATE_SLOT.into(),
                // slot index as u32
                0,
                0,
                0,
                0,
                InstructionCode::INT_8.into(),
                42,
                InstructionCode::SCOPE_END.into(),
                InstructionCode::CLOSE_AND_STORE.into(),
                InstructionCode::REMOTE_EXECUTION.into(),
                // caller (literal value 1 for test)
                InstructionCode::INT_8.into(),
                1,
                // start of block
                InstructionCode::EXECUTION_BLOCK.into(),
                // block size (21 bytes)
                24,
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
                // caller (literal value 2 for test)
                InstructionCode::GET_SLOT.into(),
                0,
                0,
                0,
                0,
                // start of block
                InstructionCode::EXECUTION_BLOCK.into(),
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
            ]
        );
    }

    #[test]
    fn assignment_to_const() {
        init_logger_debug();
        let script = "const a = 42; a = 43";
        let result = compile_script(script, CompileOptions::default());
        assert_matches!(result, Err(CompilerError::AssignmentToConst { .. }));
    }

    #[test]
    fn assignment_to_const_mut() {
        init_logger_debug();
        let script = "const a = &mut 42; a = 43";
        let result = compile_script(script, CompileOptions::default());
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
        let result = compile_script(script, CompileOptions::default());
        assert_matches!(result, Err(CompilerError::AssignmentToConst { .. }));
    }

    #[ignore = "implement type inference (precompiler)"]
    #[test]
    fn mutation_of_immutable_value() {
        init_logger_debug();
        let script = "const a = {x: 10}; a.x = 20;";
        let result = compile_script(script, CompileOptions::default());
        assert_matches!(
            result,
            Err(CompilerError::AssignmentToImmutableValue { .. })
        );
    }

    #[ignore = "implement type inference (precompiler)"]
    #[test]
    fn mutation_of_mutable_value() {
        init_logger_debug();
        let script = "const a = mut {x: 10}; a.x = 20;";
        let result = compile_script(script, CompileOptions::default());
        assert_matches!(
            result,
            Err(CompilerError::AssignmentToImmutableValue { .. })
        );
    }

    /**
     * var a = 10;
     * a = 40;
     * a += 10; // a = a + 10;
     * var a = &mut 42;;
     * a = &mut 43; // valid, new ref pointer
     * *a = 2; // internal deref assignment
     * *a += 1; // internal deref assignment with addition
     * a += 1; a = a + 1; // invalid
     * var obj = &mut {key: 42};
     * obj.key = 43; // valid, internal deref assignment
     */
    #[ignore = "implement type inference (precompiler)"]
    #[test]
    fn addition_to_immutable_ref() {
        init_logger_debug();
        let script = "const a = &42; *a += 1;";
        let result = compile_script(script, CompileOptions::default());
        assert_matches!(
            result,
            Err(CompilerError::AssignmentToImmutableReference { .. })
        );
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
        let script = "*10";
        let (res, _) =
            compile_script(script, CompileOptions::default()).unwrap();
        assert_eq!(
            res,
            vec![
                InstructionCode::DEREF.into(),
                InstructionCode::INT_8.into(),
                // integer as u8
                10,
                InstructionCode::SCOPE_END.into(),
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
                TypeSpaceInstructionCode::TYPE_LITERAL_INTEGER.into(),
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
}
