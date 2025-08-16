use crate::compiler::error::CompilerError;
use crate::global::dxb_block::DXBBlock;
use crate::global::protocol_structures::block_header::BlockHeader;
use crate::global::protocol_structures::encrypted_header::EncryptedHeader;
use crate::global::protocol_structures::routing_header;
use crate::global::protocol_structures::routing_header::RoutingHeader;

use crate::compiler::ast_parser::{
    BindingMutability, DatexExpression, DatexScriptParser, ReferenceMutability,
    TupleEntry, VariableId, VariableType, parse,
};
use crate::compiler::context::{CompilationContext, VirtualSlot};
use crate::compiler::metadata::CompileMetadata;
use crate::compiler::precompiler::{
    AstMetadata, AstWithMetadata, VariableMetadata, precompile_ast,
};
use crate::compiler::scope::CompilationScope;
use crate::global::binary_codes::{InstructionCode, InternalSlot};
use crate::values::core_values::decimal::decimal::Decimal;
use crate::values::core_values::endpoint::Endpoint;
use crate::values::value_container::ValueContainer;
use datex_core::compiler::ast_parser::Slot;
use log::info;
use std::cell::RefCell;
use std::rc::Rc;

pub mod ast_parser;
pub mod context;
pub mod error;
mod lexer;
pub mod metadata;
mod precompiler;
pub mod scope;

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StaticValueOrDXB {
    StaticValue(Option<ValueContainer>),
    Dxb(Vec<u8>),
}

impl From<Vec<u8>> for StaticValueOrDXB {
    fn from(dxb: Vec<u8>) -> Self {
        StaticValueOrDXB::Dxb(dxb)
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
    /// Determines the variable model based on the variable type and metadata.
    pub fn infer(
        variable_type: VariableType,
        variable_metadata: Option<VariableMetadata>,
        is_end_of_source_text: bool,
    ) -> Self {
        // const variables are always constant
        if variable_type == VariableType::Const {
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
        variable_type: VariableType,
        is_end_of_source_text: bool,
    ) -> Self {
        let variable_metadata =
            variable_id.and_then(|id| ast_metadata.variable_metadata(id));
        Self::infer(
            variable_type,
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
    pub var_type: VariableType,
    pub ref_mut: ReferenceMutability,
    pub binding_mut: BindingMutability,
    pub representation: VariableRepresentation,
}

impl Variable {
    pub fn new_const(
        name: String,
        mut_type: ReferenceMutability,
        slot: VirtualSlot,
    ) -> Self {
        Variable {
            name,
            var_type: VariableType::Const,
            binding_mut: BindingMutability::Immutable,
            ref_mut: mut_type,
            representation: VariableRepresentation::Constant(slot),
        }
    }

    pub fn new_variable_slot(
        name: String,
        var_type: VariableType,
        binding_mut: BindingMutability,
        ref_mut: ReferenceMutability,
        slot: VirtualSlot,
    ) -> Self {
        Variable {
            name,
            var_type,
            binding_mut,
            ref_mut,
            representation: VariableRepresentation::VariableSlot(slot),
        }
    }

    pub fn new_variable_reference(
        name: String,
        var_type: VariableType,
        ref_mut: ReferenceMutability,
        binding_mut: BindingMutability,
        variable_slot: VirtualSlot,
        container_slot: VirtualSlot,
    ) -> Self {
        Variable {
            name,
            var_type,
            ref_mut,
            binding_mut,
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

    let routing_header = RoutingHeader {
        version: 2,
        flags: routing_header::Flags::new(),
        block_size_u16: Some(0),
        block_size_u32: None,
        sender: Endpoint::LOCAL,
        receivers: routing_header::Receivers {
            flags: routing_header::ReceiverFlags::new()
                .with_has_endpoints(false)
                .with_has_pointer_id(false)
                .with_has_endpoint_keys(false),
            pointer_id: None,
            endpoints: None,
            endpoints_with_keys: None,
        },
        ..RoutingHeader::default()
    };

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
    let res = parse(datex_script)?;
    extract_static_value_from_ast(res).map(Some)
}

/// Compiles a DATEX script template text with inserted values into a DXB body
/// The value containers are passed by reference
pub fn compile_template_with_refs<'a>(
    datex_script: &'a str,
    inserted_values: &[&ValueContainer],
    options: CompileOptions<'a>,
) -> Result<(Vec<u8>, CompilationScope), CompilerError> {
    compile_template_or_return_static_value_with_refs(
        datex_script,
        inserted_values,
        false,
        options,
    )
    .map(|result| match result.0 {
        StaticValueOrDXB::StaticValue(_) => unreachable!(),
        StaticValueOrDXB::Dxb(dxb) => (dxb, result.1),
    })
}

/// Compiles a DATEX script template text with inserted values into a DXB body
/// If the script does not contain any dynamic values or operations, the static result value is
/// directly returned instead of the DXB body.
pub fn compile_script_or_return_static_value<'a>(
    datex_script: &'a str,
    options: CompileOptions<'a>,
) -> Result<(StaticValueOrDXB, CompilationScope), CompilerError> {
    compile_template_or_return_static_value_with_refs(
        datex_script,
        &[],
        true,
        options,
    )
}
/// Compiles a DATEX script template text with inserted values into a DXB body
pub fn compile_template_or_return_static_value_with_refs<'a>(
    datex_script: &'a str,
    inserted_values: &[&ValueContainer],
    return_static_value: bool,
    options: CompileOptions<'a>,
) -> Result<(StaticValueOrDXB, CompilationScope), CompilerError> {
    // shortcut if datex_script is "?" - call compile_value directly
    if datex_script == "?" {
        if inserted_values.len() != 1 {
            return Err(CompilerError::InvalidPlaceholderCount);
        }
        let result =
            compile_value(inserted_values[0]).map(StaticValueOrDXB::from)?;
        return Ok((result, options.compile_scope));
    }

    let ast = parse(datex_script)?;

    let buffer = RefCell::new(Vec::with_capacity(256));
    let compilation_context = CompilationContext::new(
        buffer,
        inserted_values,
        options.compile_scope.once,
    );

    if return_static_value {
        let scope = compile_ast(
            &compilation_context,
            ast.clone(),
            options.compile_scope,
        )?;

        if !*compilation_context.has_non_static_value.borrow() {
            if let Ok(value) = ValueContainer::try_from(ast) {
                return Ok((
                    StaticValueOrDXB::StaticValue(Some(value.clone())),
                    scope,
                ));
            }
            Ok((StaticValueOrDXB::StaticValue(None), scope))
        } else {
            // return DXB body
            Ok((
                StaticValueOrDXB::Dxb(compilation_context.buffer.take()),
                scope,
            ))
        }
    } else {
        let scope =
            compile_ast(&compilation_context, ast, options.compile_scope)?;
        // return DXB body
        Ok((
            StaticValueOrDXB::Dxb(compilation_context.buffer.take()),
            scope,
        ))
    }
}

/// Compiles a DATEX script template text with inserted values into a DXB body
pub fn compile_template<'a>(
    datex_script: &'a str,
    inserted_values: &[ValueContainer],
    options: CompileOptions<'a>,
) -> Result<(Vec<u8>, CompilationScope), CompilerError> {
    compile_template_with_refs(
        datex_script,
        &inserted_values.iter().collect::<Vec<_>>(),
        options,
    )
}

pub fn compile_value(value: &ValueContainer) -> Result<Vec<u8>, CompilerError> {
    let buffer = RefCell::new(Vec::with_capacity(256));
    let compilation_scope = CompilationContext::new(buffer, &[], true);

    compilation_scope.insert_value_container(value);

    Ok(compilation_scope.buffer.take())
}

/// Tries to extract a static value from a DATEX expression AST.
/// If the expression is not a static value (e.g., contains a placeholder or dynamic operation),
/// it returns an error.
fn extract_static_value_from_ast(
    ast: DatexExpression,
) -> Result<ValueContainer, CompilerError> {
    if let DatexExpression::Placeholder = ast {
        return Err(CompilerError::NonStaticValue);
    }
    ValueContainer::try_from(ast).map_err(|_| CompilerError::NonStaticValue)
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

pub fn compile_ast(
    compilation_context: &CompilationContext,
    ast: DatexExpression,
    mut scope: CompilationScope,
) -> Result<CompilationScope, CompilerError> {
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
                ast,
                precompiler_data.ast_metadata.clone(),
                &mut precompiler_data.precompiler_scope_stack.borrow_mut(),
            )?
        } else {
            // if no precompiler data, just use the AST with default metadata
            AstWithMetadata::new_without_metadata(ast)
        };

    compile_ast_with_metadata(compilation_context, ast_with_metadata, scope)
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
    match ast_with_metadata.ast {
        DatexExpression::Integer(int) => {
            compilation_context.insert_int(int.0.as_i64().unwrap());
        }
        DatexExpression::Decimal(decimal) => match &decimal {
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
        DatexExpression::Text(text) => {
            compilation_context.insert_text(&text);
        }
        DatexExpression::Boolean(boolean) => {
            compilation_context.insert_boolean(boolean);
        }
        DatexExpression::Endpoint(endpoint) => {
            compilation_context.insert_endpoint(&endpoint);
        }
        DatexExpression::Null => {
            compilation_context.append_binary_code(InstructionCode::NULL);
        }
        DatexExpression::Array(array) => {
            compilation_context
                .append_binary_code(InstructionCode::ARRAY_START);
            for item in array {
                scope = compile_expression(
                    compilation_context,
                    AstWithMetadata::new(item, &metadata),
                    CompileMetadata::default(),
                    scope,
                )?;
            }
            compilation_context.append_binary_code(InstructionCode::SCOPE_END);
        }
        DatexExpression::Tuple(tuple) => {
            compilation_context
                .append_binary_code(InstructionCode::TUPLE_START);
            for entry in tuple {
                match entry {
                    TupleEntry::KeyValue(key, value) => {
                        scope = compile_key_value_entry(
                            compilation_context,
                            key,
                            value,
                            &metadata,
                            scope,
                        )?;
                    }
                    TupleEntry::Value(value) => {
                        scope = compile_expression(
                            compilation_context,
                            AstWithMetadata::new(value, &metadata),
                            CompileMetadata::default(),
                            scope,
                        )?;
                    }
                }
            }
            compilation_context.append_binary_code(InstructionCode::SCOPE_END);
        }
        DatexExpression::Object(object) => {
            compilation_context
                .append_binary_code(InstructionCode::OBJECT_START);
            for (key, value) in object {
                // compile key and value
                scope = compile_key_value_entry(
                    compilation_context,
                    key,
                    value,
                    &metadata,
                    scope,
                )?;
            }
            compilation_context.append_binary_code(InstructionCode::SCOPE_END);
        }

        DatexExpression::Placeholder => {
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
        DatexExpression::Statements(mut statements) => {
            compilation_context.mark_has_non_static_value();
            // if single statement and not terminated, just compile the expression
            if statements.len() == 1 && !statements[0].is_terminated {
                scope = compile_expression(
                    compilation_context,
                    AstWithMetadata::new(
                        statements.remove(0).expression,
                        &metadata,
                    ),
                    CompileMetadata::default(),
                    scope,
                )?;
            } else {
                // if not outer context, new scope
                let mut child_scope = if !meta.is_outer_context() {
                    compilation_context
                        .append_binary_code(InstructionCode::SCOPE_START);
                    scope.push()
                } else {
                    scope
                };
                for statement in statements {
                    child_scope = compile_expression(
                        compilation_context,
                        AstWithMetadata::new(statement.expression, &metadata),
                        CompileMetadata::default(),
                        child_scope,
                    )?;
                    // if statement is terminated, append close and store
                    if statement.is_terminated {
                        compilation_context.append_binary_code(
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
                        compilation_context
                            .append_binary_code(InstructionCode::DROP_SLOT);
                        // insert virtual slot address for dropping
                        compilation_context
                            .insert_virtual_slot_address(slot_address);
                    }
                    compilation_context
                        .append_binary_code(InstructionCode::SCOPE_END);
                } else {
                    scope = child_scope;
                }
            }
        }

        // operations (add, subtract, multiply, divide, etc.)
        DatexExpression::BinaryOperation(operator, a, b) => {
            compilation_context.mark_has_non_static_value();
            // append binary code for operation if not already current binary operator
            compilation_context
                .append_binary_code(InstructionCode::from(&operator));
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
        DatexExpression::ApplyChain(val, operands) => {
            compilation_context.mark_has_non_static_value();
            // TODO #150
        }

        // variables
        // declaration
        DatexExpression::VariableDeclaration(
            id,
            var_type,
            binding_mut,
            ref_mut,
            name,
            expression,
        ) => {
            compilation_context.mark_has_non_static_value();

            // allocate new slot for variable
            let virtual_slot_addr = scope.get_next_virtual_slot();
            compilation_context
                .append_binary_code(InstructionCode::ALLOCATE_SLOT);
            compilation_context.insert_virtual_slot_address(
                VirtualSlot::local(virtual_slot_addr),
            );
            // create reference if internally mutable
            if ref_mut == ReferenceMutability::Mutable {
                compilation_context
                    .append_binary_code(InstructionCode::CREATE_REF);
            }
            // compile expression
            scope = compile_expression(
                compilation_context,
                AstWithMetadata::new(*expression, &metadata),
                CompileMetadata::default(),
                scope,
            )?;

            let variable_model =
                VariableModel::infer_from_ast_metadata_and_type(
                    &metadata.borrow(),
                    id,
                    var_type,
                    compilation_context.is_end_of_source_text,
                );
            info!("variable model for {name}: {variable_model:?}");

            // create new variable depending on the model
            let variable = match variable_model {
                VariableModel::VariableReference => {
                    // scope end
                    compilation_context
                        .append_binary_code(InstructionCode::SCOPE_END);
                    // allocate an additional slot with a reference to the variable
                    let virtual_slot_addr_for_var =
                        scope.get_next_virtual_slot();
                    compilation_context
                        .append_binary_code(InstructionCode::ALLOCATE_SLOT);
                    compilation_context.insert_virtual_slot_address(
                        VirtualSlot::local(virtual_slot_addr_for_var),
                    );
                    // indirect reference to the variable
                    compilation_context
                        .append_binary_code(InstructionCode::CREATE_REF);
                    // append binary code to load variable
                    compilation_context
                        .append_binary_code(InstructionCode::GET_SLOT);
                    compilation_context.insert_virtual_slot_address(
                        VirtualSlot::local(virtual_slot_addr),
                    );

                    Variable::new_variable_reference(
                        name.clone(),
                        var_type,
                        ref_mut,
                        binding_mut,
                        VirtualSlot::local(virtual_slot_addr_for_var),
                        VirtualSlot::local(virtual_slot_addr),
                    )
                }
                VariableModel::Constant => Variable::new_const(
                    name.clone(),
                    ref_mut,
                    VirtualSlot::local(virtual_slot_addr),
                ),
                VariableModel::VariableSlot => Variable::new_variable_slot(
                    name.clone(),
                    var_type,
                    binding_mut,
                    ref_mut,
                    VirtualSlot::local(virtual_slot_addr),
                ),
            };

            scope.register_variable_slot(variable);

            compilation_context.append_binary_code(InstructionCode::SCOPE_END);
        }

        // assignment
        DatexExpression::VariableAssignment(id, name, expression) => {
            compilation_context.mark_has_non_static_value();
            // get variable slot address
            let (virtual_slot, var_type, mut_type) = scope
                .resolve_variable_name_to_virtual_slot(&name)
                .ok_or_else(|| {
                    CompilerError::UndeclaredVariable(name.clone())
                })?;

            // if const, return error
            if var_type == VariableType::Const {
                return Err(CompilerError::AssignmentToConst(name.clone()));
            }

            // append binary code to load variable
            info!(
                "append variable virtual slot: {virtual_slot:?}, name: {name}"
            );
            compilation_context.append_binary_code(InstructionCode::SET_SLOT);
            compilation_context.insert_virtual_slot_address(virtual_slot);
            // compile expression
            scope = compile_expression(
                compilation_context,
                AstWithMetadata::new(*expression, &metadata),
                CompileMetadata::default(),
                scope,
            )?;
            // close assignment scope
            compilation_context.append_binary_code(InstructionCode::SCOPE_END);
        }

        // variable access
        DatexExpression::Variable(id, name) => {
            compilation_context.mark_has_non_static_value();
            // get variable slot address
            let (virtual_slot, ..) = scope
                .resolve_variable_name_to_virtual_slot(&name)
                .ok_or_else(|| {
                    CompilerError::UndeclaredVariable(name.clone())
                })?;
            // append binary code to load variable
            compilation_context.append_binary_code(InstructionCode::GET_SLOT);
            compilation_context.insert_virtual_slot_address(virtual_slot);
        }

        // remote execution
        DatexExpression::RemoteExecution(caller, script) => {
            compilation_context.mark_has_non_static_value();

            // insert remote execution code
            compilation_context
                .append_binary_code(InstructionCode::REMOTE_EXECUTION);
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
                &[],
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
                .append_binary_code(InstructionCode::EXECUTION_BLOCK);
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
        DatexExpression::Slot(Slot::Named(name)) => {
            match name.as_str() {
                "endpoint" => {
                    compilation_context
                        .append_binary_code(InstructionCode::GET_SLOT);
                    compilation_context
                        .append_u32(InternalSlot::ENDPOINT as u32);
                }
                _ => {
                    // invalid slot name
                    return Err(CompilerError::InvalidSlotName(name.clone()));
                }
            }
        }

        _ => return Err(CompilerError::UnexpectedTerm(ast_with_metadata.ast)),
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
    match key {
        // text -> insert key string
        DatexExpression::Text(text) => {
            compilation_scope.insert_key_string(&text);
        }
        // other -> insert key as dynamic
        _ => {
            compilation_scope
                .append_binary_code(InstructionCode::KEY_VALUE_DYNAMIC);
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
    use super::{
        CompilationContext, CompilationScope, CompileOptions, StaticValueOrDXB,
        compile_ast, compile_script, compile_script_or_return_static_value,
        compile_template,
    };
    use std::assert_matches::assert_matches;
    use std::cell::RefCell;
    use std::io::Read;
    use std::vec;

    use crate::compiler::ast_parser::parse;
    use crate::values::core_values::integer::integer::Integer;
    use crate::{
        global::binary_codes::InstructionCode, logger::init_logger_debug,
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

    fn get_compilation_scope(script: &str) -> CompilationContext {
        let ast = parse(script);
        let ast = ast.unwrap();
        let buffer = RefCell::new(Vec::with_capacity(256));
        let compilation_scope = CompilationContext::new(buffer, &[], true);
        compile_ast(&compilation_scope, ast, CompilationScope::default())
            .unwrap();
        compilation_scope
    }

    #[test]
    fn test_simple_multiplication() {
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
    fn test_simple_multiplication_close() {
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
    fn test_is_operator() {
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
                InstructionCode::CREATE_REF.into(),
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
                InstructionCode::CREATE_REF.into(),
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
    fn test_equality_operator() {
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
    fn test_simple_addition() {
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
    fn test_multi_addition() {
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
    fn test_mixed_calculation() {
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
    fn test_complex_addition() {
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
    fn test_complex_addition_and_subtraction() {
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

    // Test for integer/u8
    #[test]
    fn test_integer_u8() {
        init_logger_debug();
        let val: u8 = 42;
        let datex_script = format!("{val}"); // 42
        let result = compile_and_log(&datex_script);
        assert_eq!(result, vec![InstructionCode::INT_8.into(), val,]);
    }

    // Test for decimal
    #[test]
    fn test_decimal() {
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
    fn test_short_text() {
        init_logger_debug();
        let val = "unyt";
        let datex_script = format!("\"{val}\""); // "42"
        let result = compile_and_log(&datex_script);
        let mut expected: Vec<u8> =
            vec![InstructionCode::SHORT_TEXT.into(), val.len() as u8];
        expected.extend(val.bytes());
        assert_eq!(result, expected);
    }

    // Test empty array
    #[test]
    fn test_empty_array() {
        init_logger_debug();
        let datex_script = "[]";
        let result = compile_and_log(datex_script);
        let expected: Vec<u8> = vec![
            InstructionCode::ARRAY_START.into(),
            InstructionCode::SCOPE_END.into(),
        ];
        assert_eq!(result, expected);
    }

    // Test array with single element
    #[test]
    fn test_single_element_array() {
        init_logger_debug();
        let datex_script = "[42]";
        let result = compile_and_log(datex_script);
        assert_eq!(
            result,
            vec![
                InstructionCode::ARRAY_START.into(),
                InstructionCode::INT_8.into(),
                42,
                InstructionCode::SCOPE_END.into(),
            ]
        );
    }

    // Test array with multiple elements
    #[test]
    fn test_multi_element_array() {
        init_logger_debug();
        let datex_script = "[1, 2, 3]";
        let result = compile_and_log(datex_script);
        assert_eq!(
            result,
            vec![
                InstructionCode::ARRAY_START.into(),
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

    // Test nested arrays
    #[test]
    fn test_nested_arrays() {
        init_logger_debug();
        let datex_script = "[1, [2, 3], 4]";
        let result = compile_and_log(datex_script);
        assert_eq!(
            result,
            vec![
                InstructionCode::ARRAY_START.into(),
                InstructionCode::INT_8.into(),
                1,
                InstructionCode::ARRAY_START.into(),
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

    // Test array with expressions inside
    #[test]
    fn test_array_with_expressions() {
        init_logger_debug();
        let datex_script = "[1 + 2, 3 * 4]";
        let result = compile_and_log(datex_script);
        assert_eq!(
            result,
            vec![
                InstructionCode::ARRAY_START.into(),
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

    // Test array with mixed expressions
    #[test]
    fn test_array_with_mixed_expressions() {
        init_logger_debug();
        let datex_script = "[1, 2, 3 + 4]";
        let result = compile_and_log(datex_script);
        assert_eq!(
            result,
            vec![
                InstructionCode::ARRAY_START.into(),
                InstructionCode::INT_8.into(),
                1,
                InstructionCode::INT_8.into(),
                2,
                InstructionCode::ADD.into(),
                InstructionCode::INT_8.into(),
                3,
                InstructionCode::INT_8.into(),
                4,
                InstructionCode::SCOPE_END.into(),
            ]
        );
    }

    // Test tuple
    #[test]
    fn test_tuple() {
        init_logger_debug();
        let datex_script = "(1, 2, 3)";
        let result = compile_and_log(datex_script);
        assert_eq!(
            result,
            vec![
                InstructionCode::TUPLE_START.into(),
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

    // Nested tuple
    #[test]
    fn test_nested_tuple() {
        init_logger_debug();
        let datex_script = "(1, (2, 3), 4)";
        let result = compile_and_log(datex_script);
        assert_eq!(
            result,
            vec![
                InstructionCode::TUPLE_START.into(),
                InstructionCode::INT_8.into(),
                1,
                InstructionCode::TUPLE_START.into(),
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

    // Tuple without parentheses
    #[test]
    fn test_tuple_without_parentheses() {
        init_logger_debug();
        let datex_script = "1, 2, 3";
        let result = compile_and_log(datex_script);
        assert_eq!(
            result,
            vec![
                InstructionCode::TUPLE_START.into(),
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

    // key-value pair
    #[test]
    fn test_key_value_tuple() {
        init_logger_debug();
        let datex_script = "key: 42";
        let result = compile_and_log(datex_script);
        let expected = vec![
            InstructionCode::TUPLE_START.into(),
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

    // key-value pair with string key
    #[test]
    fn test_key_value_string() {
        init_logger_debug();
        let datex_script = "\"key\": 42";
        let result = compile_and_log(datex_script);
        let expected = vec![
            InstructionCode::TUPLE_START.into(),
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

    // key-value pair with integer key
    #[test]
    fn test_key_value_integer() {
        init_logger_debug();
        let datex_script = "10: 42";
        let result = compile_and_log(datex_script);
        let expected = vec![
            InstructionCode::TUPLE_START.into(),
            InstructionCode::KEY_VALUE_DYNAMIC.into(),
            InstructionCode::INT_8.into(),
            10,
            InstructionCode::INT_8.into(),
            42,
            InstructionCode::SCOPE_END.into(),
        ];
        assert_eq!(result, expected);
    }

    // key-value pair with long text key (>255 bytes)
    #[test]
    fn test_key_value_long_text() {
        init_logger_debug();
        let long_key = "a".repeat(300);
        let datex_script = format!("\"{long_key}\": 42");
        let result = compile_and_log(&datex_script);
        let mut expected: Vec<u8> = vec![
            InstructionCode::TUPLE_START.into(),
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

    // dynamic key-value pair
    #[test]
    fn test_dynamic_key_value() {
        init_logger_debug();
        let datex_script = "(1 + 2): 42";
        let result = compile_and_log(datex_script);
        let expected = [
            InstructionCode::TUPLE_START.into(),
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

    // multiple key-value pairs
    #[test]
    fn test_multiple_key_value_pairs() {
        init_logger_debug();
        let datex_script = "key: 42, 4: 43, (1 + 2): 44";
        let result = compile_and_log(datex_script);
        let expected = vec![
            InstructionCode::TUPLE_START.into(),
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

    // key value pair with parentheses
    #[test]
    fn test_key_value_with_parentheses() {
        init_logger_debug();
        let datex_script = "(key: 42)";
        let result = compile_and_log(datex_script);
        let expected = vec![
            InstructionCode::TUPLE_START.into(),
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

    // empty object
    #[test]
    fn test_empty_object() {
        init_logger_debug();
        let datex_script = "{}";
        let result = compile_and_log(datex_script);
        let expected: Vec<u8> = vec![
            InstructionCode::OBJECT_START.into(),
            InstructionCode::SCOPE_END.into(),
        ];
        assert_eq!(result, expected);
    }

    // object with single key-value pair
    #[test]
    fn test_single_key_value_object() {
        init_logger_debug();
        let datex_script = "{key: 42}";
        let result = compile_and_log(datex_script);
        let expected = vec![
            InstructionCode::OBJECT_START.into(),
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

    // object with multiple key-value pairs
    #[test]
    fn test_multi_key_value_object() {
        init_logger_debug();
        let datex_script = "{key1: 42, \"key2\": 43, 'key3': 44}";
        let result = compile_and_log(datex_script);
        let expected = vec![
            InstructionCode::OBJECT_START.into(),
            InstructionCode::KEY_VALUE_SHORT_TEXT.into(),
            4, // length of "key1"
            b'k',
            b'e',
            b'y',
            b'1',
            InstructionCode::INT_8.into(),
            42,
            InstructionCode::KEY_VALUE_SHORT_TEXT.into(),
            4, // length of "key2"
            b'k',
            b'e',
            b'y',
            b'2',
            InstructionCode::INT_8.into(),
            43,
            InstructionCode::KEY_VALUE_SHORT_TEXT.into(),
            4, // length of "key3"
            b'k',
            b'e',
            b'y',
            b'3',
            InstructionCode::INT_8.into(),
            44,
            InstructionCode::SCOPE_END.into(),
        ];
        assert_eq!(result, expected);
    }

    #[test]
    fn test_allocate_slot() {
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
    fn test_allocate_slot_with_value() {
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
    fn test_allocate_scoped_slots() {
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
    fn test_allocate_scoped_slots_with_parent_variables() {
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
    fn test_allocate_ref() {
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
                InstructionCode::CREATE_REF.into(),
                InstructionCode::INT_8.into(),
                42,
                InstructionCode::SCOPE_END.into(),
            ]
        );
    }

    #[test]
    fn test_read_ref() {
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
                InstructionCode::CREATE_REF.into(),
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
    fn test_compile() {
        init_logger_debug();
        let result = compile_template(
            "? + ?",
            &[1.into(), 2.into()],
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
    fn test_compile_macro() {
        init_logger_debug();
        let a = 1;
        let result = compile!("?", a);
        assert_eq!(result.unwrap().0, vec![InstructionCode::INT_8.into(), 1,]);
    }

    #[test]
    fn test_compile_macro_multi() {
        init_logger_debug();
        let result = compile!("? + ?", 1, 2);
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
    fn test_json_to_dxb_large_file() {
        let json = get_json_test_string("test2.json");
        println!("JSON file read");
        let (dxb, _) = compile_script(&json, CompileOptions::default())
            .expect("Failed to parse JSON string");
        println!("DXB: {:?}", dxb.len());
    }

    #[test]
    fn test_static_value_detection() {
        init_logger_debug();

        // non-static
        let script = "1 + 2";
        let compilation_scope = get_compilation_scope(script);
        assert!(*compilation_scope.has_non_static_value.borrow());

        let script = "1 2";
        let compilation_scope = get_compilation_scope(script);
        assert!(*compilation_scope.has_non_static_value.borrow());

        let script = "1;2";
        let compilation_scope = get_compilation_scope(script);
        assert!(*compilation_scope.has_non_static_value.borrow());

        let script = r#"{("x" + "y"): 1}"#;
        let compilation_scope = get_compilation_scope(script);
        assert!(*compilation_scope.has_non_static_value.borrow());

        // static
        let script = "1";
        let compilation_scope = get_compilation_scope(script);
        assert!(!*compilation_scope.has_non_static_value.borrow());

        let script = "[]";
        let compilation_scope = get_compilation_scope(script);
        assert!(!*compilation_scope.has_non_static_value.borrow());

        let script = "{}";
        let compilation_scope = get_compilation_scope(script);
        assert!(!*compilation_scope.has_non_static_value.borrow());

        let script = "[1,2,3]";
        let compilation_scope = get_compilation_scope(script);
        assert!(!*compilation_scope.has_non_static_value.borrow());

        let script = "{a: 2}";
        let compilation_scope = get_compilation_scope(script);
        assert!(!*compilation_scope.has_non_static_value.borrow());
    }

    #[test]
    fn test_compile_auto_static_value_detection() {
        let script = "1";
        let (res, _) = compile_script_or_return_static_value(
            script,
            CompileOptions::default(),
        )
        .unwrap();
        assert_eq!(
            res,
            StaticValueOrDXB::StaticValue(Some(Integer::from(1).into()))
        );

        let script = "1 + 2";
        let (res, _) = compile_script_or_return_static_value(
            script,
            CompileOptions::default(),
        )
        .unwrap();
        assert_eq!(
            res,
            StaticValueOrDXB::Dxb(vec![
                InstructionCode::ADD.into(),
                InstructionCode::INT_8.into(),
                1,
                InstructionCode::INT_8.into(),
                2,
            ])
        );
    }

    #[test]
    fn test_remote_execution() {
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
    fn test_remote_execution_expression() {
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
    fn test_remote_execution_injected_const() {
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
    fn test_remote_execution_injected_var() {
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
    fn test_remote_execution_injected_consts() {
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
    fn test_remote_execution_shadow_const() {
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
    fn test_remote_execution_nested() {
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
    fn test_remote_execution_nested2() {
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
    fn test_assignment_to_const() {
        init_logger_debug();
        let script = "const a = 42; a = 43";
        let result = compile_script(script, CompileOptions::default());
        assert_matches!(result, Err(CompilerError::AssignmentToConst { .. }));
    }

    #[test]
    fn test_assignment_to_const_mut() {
        init_logger_debug();
        let script = "const a = &mut 42; a = 43";
        let result = compile_script(script, CompileOptions::default());
        assert_matches!(result, Err(CompilerError::AssignmentToConst { .. }));
    }

    // WIP
    #[test]
    fn test_addition_to_const_ref() {
        init_logger_debug();
        let script = "const a = &mut 42; a += 1";
        let result = compile_script(script, CompileOptions::default());
        assert_matches!(result, Ok(_));
    }

    // WIP
    #[test]
    fn test_addition_to_immutable_value() {
        init_logger_debug();
        let script = "var a = 42; a += 1";
        let result = compile_script(script, CompileOptions::default());
        assert_matches!(result, Err(CompilerError::AssignmentToConst { .. })); // AssignmentToImmutableValue
    }

    // WIP
    #[test]
    fn test_addition_to_immutable_ref() {
        init_logger_debug();
        let script = "const a = &42; a += 1";
        let result = compile_script(script, CompileOptions::default());
        assert_matches!(result, Err(CompilerError::AssignmentToConst { .. })); // AssignmentToImmutableReference
    }

    #[test]
    fn test_slot_endpoint() {
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
}
