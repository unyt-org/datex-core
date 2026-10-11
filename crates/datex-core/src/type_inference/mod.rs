//! This module contains the implementation of the type inference system based on a [RichAst].
use crate::{
    ast::resolved_variable::ResolvedVariable,
    global::operators::{BinaryOperator, LogicalUnaryOperator, UnaryOperator},
    type_inference::options::ErrorHandling,
    types::{
        type_definition::{
            TypeDefinition, callable::CallableTypeDefinition,
            list::ListTypeDefinition,
        },
        type_definition_with_metadata::LocalOwnership,
    },
    values::core_values::{boolean::Boolean, text::Text},
};

use crate::{
    ast::{
        expressions::{
            Apply, BinaryOperation, CallableDeclaration, ComparisonOperation,
            Conditional, CreateShared, DatexExpression, DatexExpressionData,
            DeriveRef, DeriveSharedRef, EntityDeclarationExpression,
            GenericInstantiation, List, Map, PropertyAccess,
            PropertyAssignment, RangeDeclaration, RemoteExecution,
            RequestSharedRef, StackAssignment, Statements,
            TypeDeclarationExpression, UnaryOperation, Unbox, UnboxAssignment,
            ValueAccessType, VariableAccess, VariableAssignment,
            VariableDeclaration, VariantAccess,
        },
        type_expressions::{
            CallableTypeExpression, FixedSizeList, GenericAccess, Intersection,
            SliceList, StructuralList, StructuralMap, TypeExpression,
            TypeVariantAccess, Union,
        },
    },
    compiler::precompiler::precompiled_ast::{AstMetadata, RichAst},
    global::stack_index::StackIndex,
    libs::core::{
        core_lib_id::CoreLibId,
        type_id::{CoreLibBaseTypeId, CoreLibTypeId},
    },
    prelude::*,
    runtime::cache::shared_references_cache::SharedReferencesCache,
    shared_values::{
        PointerAddress, ReferenceMutability, SharedContainer,
        SharedContainerOwnership,
    },
    type_inference::{
        error::{
            DetailedTypeErrors, SimpleOrDetailedTypeError, SpannedTypeError,
        },
        options::InferExpressionTypeOptions,
    },
    types::{
        error::TypeError,
        literal_type_definition::LiteralTypeDefinition,
        traits::type_match::{TypeSubset, TypeSuperset},
        r#type::Type,
        type_definition::{
            range::RangeTypeDefinition, union::UnionTypeDefinition,
        },
        type_definition_with_metadata::{
            LocalMutability, TypeDefinitionWithMetadata, TypeMetadata,
        },
    },
    values::{
        core_value::CoreValue,
        core_values::{
            decimal::{Decimal, typed_decimal::TypedDecimal},
            endpoint::Endpoint,
            integer::{Integer, typed_integer::TypedInteger},
        },
    },
    visitor::{
        VisitAction,
        expression::{ExpressionVisitor, visitable::ExpressionVisitResult},
        type_expression::{
            TypeExpressionVisitor, visitable::TypeExpressionVisitResult,
        },
    },
};
use core::{cell::RefCell, ops::Range, panic};

pub mod error;
pub mod options;

// TODO #617: refactor InferOutcome to a struct containing type, errors and warnings
#[derive(Debug)]
pub enum InferOutcome {
    Ok(Type),
    OkWithErrors {
        ty: Type,
        errors: DetailedTypeErrors,
    },
}
impl From<InferOutcome> for Type {
    fn from(outcome: InferOutcome) -> Self {
        match outcome {
            InferOutcome::Ok(ty) => ty,
            InferOutcome::OkWithErrors { ty, .. } => ty,
        }
    }
}

impl InferOutcome {
    pub fn to_type(self) -> Type {
        match self {
            InferOutcome::Ok(ty) => ty,
            InferOutcome::OkWithErrors { ty, .. } => ty,
        }
    }
    pub fn unwrap_err(self) -> DetailedTypeErrors {
        match self {
            InferOutcome::Ok(_ty) => {
                panic!("Expected errors, got successful type inference")
            }
            InferOutcome::OkWithErrors { errors, .. } => errors,
        }
    }
}

pub fn infer_expression_type_simple_error(
    rich_ast: &mut RichAst,
    memory: &SharedReferencesCache,
) -> Result<Type, SpannedTypeError> {
    match infer_expression_type(
        rich_ast,
        InferExpressionTypeOptions {
            detailed_errors: false,
            error_handling: ErrorHandling::FailFast,
        },
        memory,
    ) {
        Ok(InferOutcome::Ok(ty)) => Ok(ty),
        Ok(InferOutcome::OkWithErrors { ty, .. }) => Ok(ty),
        Err(SimpleOrDetailedTypeError::Simple(e)) => Err(e),
        Err(SimpleOrDetailedTypeError::Detailed(_)) => unreachable!(),
    }
}

pub fn infer_expression_type_detailed_errors(
    rich_ast: &mut RichAst,
    memory: &SharedReferencesCache,
) -> Result<Type, DetailedTypeErrors> {
    match infer_expression_type(
        rich_ast,
        InferExpressionTypeOptions {
            detailed_errors: true,
            error_handling: ErrorHandling::Collect,
        },
        memory,
    ) {
        Ok(InferOutcome::Ok(ty)) => Ok(ty),
        Ok(InferOutcome::OkWithErrors { .. }) => unreachable!(),
        Err(SimpleOrDetailedTypeError::Detailed(e)) => Err(e),
        Err(SimpleOrDetailedTypeError::Simple(_)) => unreachable!(),
    }
}

pub fn infer_expression_type_with_errors(
    rich_ast: &mut RichAst,
    memory: &SharedReferencesCache,
) -> InferOutcome {
    infer_expression_type(
        rich_ast,
        InferExpressionTypeOptions {
            detailed_errors: true,
            error_handling: ErrorHandling::CollectAndReturnType,
        },
        memory,
    )
    .unwrap()
}

/// Infers the type of an expression as precisely as possible.
/// Uses cached type information if available.
fn infer_expression_type(
    rich_ast: &mut RichAst,
    options: InferExpressionTypeOptions,
    memory: &SharedReferencesCache,
) -> Result<InferOutcome, SimpleOrDetailedTypeError> {
    TypeInference::new(rich_ast.metadata.clone(), memory)
        .infer(&mut rich_ast.ast, options)
}
pub struct TypeInference<'a> {
    errors: Option<DetailedTypeErrors>,
    metadata: Rc<RefCell<AstMetadata>>,
    memory: &'a SharedReferencesCache,
}

impl<'a> TypeInference<'a> {
    pub fn new(
        metadata: Rc<RefCell<AstMetadata>>,
        memory: &'a SharedReferencesCache,
    ) -> Self {
        TypeInference {
            metadata,
            errors: None,
            memory,
        }
    }

    pub fn infer(
        &mut self,
        ast: &mut DatexExpression,
        options: InferExpressionTypeOptions,
    ) -> Result<InferOutcome, SimpleOrDetailedTypeError> {
        // Enable error collection if needed
        if options.detailed_errors {
            self.errors = Some(DetailedTypeErrors { errors: vec![] });
        } else {
            self.errors = None;
        }

        let result = self.infer_expression(ast);
        let collected_errors = self.errors.take();
        let has_errors = collected_errors
            .as_ref()
            .map(|e| e.has_errors())
            .unwrap_or(false);

        match options.error_handling {
            ErrorHandling::FailFast => result
                .map(InferOutcome::Ok)
                .map_err(SimpleOrDetailedTypeError::from),

            ErrorHandling::Collect => {
                if has_errors {
                    Err(SimpleOrDetailedTypeError::Detailed(
                        collected_errors.unwrap(),
                    ))
                } else {
                    result
                        .map(InferOutcome::Ok)
                        .map_err(SimpleOrDetailedTypeError::from)
                }
            }

            ErrorHandling::CollectAndReturnType => {
                let ty = result
                    .unwrap_or_else(|_| Type::core(CoreLibBaseTypeId::Never));
                if has_errors {
                    Ok(InferOutcome::OkWithErrors {
                        ty,
                        errors: collected_errors.unwrap(),
                    })
                } else {
                    Ok(InferOutcome::Ok(ty))
                }
            }
        }
    }

    fn infer_expression(
        &mut self,
        expr: &mut DatexExpression,
    ) -> Result<Type, SpannedTypeError> {
        self.visit_datex_expression(expr)?;
        Ok(expr
            .ty
            .clone()
            .unwrap_or_else(|| Type::core(CoreLibBaseTypeId::Never)))
    }

    fn infer_type_expression(
        &mut self,
        type_expr: &mut TypeExpression,
    ) -> Result<Type, SpannedTypeError> {
        self.visit_type_expression(type_expr)?;
        Ok(type_expr
            .ty
            .clone()
            .unwrap_or_else(|| Type::core(CoreLibBaseTypeId::Never)))
    }

    fn variable_type(&self, id: usize) -> Option<Type> {
        self.metadata
            .borrow()
            .variable_metadata(id)
            .and_then(|meta| meta.var_type.clone())
    }
    fn update_variable_type(&mut self, id: usize, var_type: Type) {
        if let Some(var_meta) =
            self.metadata.borrow_mut().variable_metadata_mut(id)
        {
            var_meta.var_type = Some(var_type);
        } else {
            panic!("Variable metadata not found for id {}", id);
        }
    }
    fn record_error(
        &mut self,
        error: SpannedTypeError,
    ) -> Result<VisitAction<DatexExpression>, SpannedTypeError> {
        if let Some(collected_errors) = &mut self.errors {
            let action = match *error.error {
                TypeError::Unimplemented(_) => {
                    VisitAction::SetTypeRecurseChildNodes(Type::core(
                        CoreLibBaseTypeId::Never,
                    ))
                }
                _ => VisitAction::SetTypeSkipChildren(Type::core(
                    CoreLibBaseTypeId::Never,
                )),
            };
            collected_errors.errors.push(error);
            Ok(action)
        } else {
            Err(error)
        }
    }
}

fn mark_type_definition<E>(
    definition: TypeDefinition,
) -> Result<VisitAction<E>, SpannedTypeError> {
    mark_type(Type::Definition(definition.into()))
}

fn mark_literal_type<E>(
    definition: LiteralTypeDefinition,
) -> Result<VisitAction<E>, SpannedTypeError> {
    mark_type(Type::Definition(definition.into()))
}
fn mark_type<E>(ty: Type) -> Result<VisitAction<E>, SpannedTypeError> {
    Ok(VisitAction::SetTypeSkipChildren(ty))
}

fn mark_never<E>() -> Result<VisitAction<E>, SpannedTypeError> {
    mark_type(Type::core(CoreLibBaseTypeId::Never))
}

fn mark_type_or_never<E>(
    maybe_type: Option<Type>,
) -> Result<VisitAction<E>, SpannedTypeError> {
    mark_type(
        maybe_type.unwrap_or_else(|| Type::core(CoreLibBaseTypeId::Never)),
    )
}

impl<'a> TypeExpressionVisitor<SpannedTypeError> for TypeInference<'a> {
    fn visit_integer_type(
        &mut self,
        integer: &mut Integer,
        _: &Range<usize>,
    ) -> TypeExpressionVisitResult<SpannedTypeError> {
        mark_literal_type(LiteralTypeDefinition::Integer(integer.clone()))
    }
    fn visit_typed_integer_type(
        &mut self,
        typed_integer: &mut TypedInteger,
        _: &Range<usize>,
    ) -> TypeExpressionVisitResult<SpannedTypeError> {
        mark_literal_type(LiteralTypeDefinition::TypedInteger(
            typed_integer.clone(),
        ))
    }
    fn visit_decimal_type(
        &mut self,
        decimal: &mut Decimal,
        _: &Range<usize>,
    ) -> TypeExpressionVisitResult<SpannedTypeError> {
        mark_literal_type(LiteralTypeDefinition::Decimal(decimal.clone()))
    }
    fn visit_typed_decimal_type(
        &mut self,
        decimal: &mut TypedDecimal,
        _: &Range<usize>,
    ) -> TypeExpressionVisitResult<SpannedTypeError> {
        mark_literal_type(LiteralTypeDefinition::TypedDecimal(decimal.clone()))
    }
    fn visit_boolean_type(
        &mut self,
        boolean: &mut Boolean,
        _: &Range<usize>,
    ) -> TypeExpressionVisitResult<SpannedTypeError> {
        mark_literal_type(LiteralTypeDefinition::Boolean(boolean.clone()))
    }
    fn visit_text_type(
        &mut self,
        text: &mut Text,
        _: &Range<usize>,
    ) -> TypeExpressionVisitResult<SpannedTypeError> {
        mark_literal_type(LiteralTypeDefinition::Text(text.clone()))
    }
    fn visit_endpoint_type(
        &mut self,
        endpoint: &mut Endpoint,
        _: &Range<usize>,
    ) -> TypeExpressionVisitResult<SpannedTypeError> {
        mark_literal_type(LiteralTypeDefinition::Endpoint(endpoint.clone()))
    }
    fn visit_union_type(
        &mut self,
        union: &mut Union,
        _: &Range<usize>,
    ) -> TypeExpressionVisitResult<SpannedTypeError> {
        let members = union
            .0
            .iter_mut()
            .map(|member| self.infer_type_expression(member))
            .collect::<Result<Vec<_>, _>>()?;
        mark_type(Type::from(TypeDefinition::Union(UnionTypeDefinition(
            members,
        ))))
    }
    fn visit_intersection_type(
        &mut self,
        intersection: &mut Intersection,
        _: &Range<usize>,
    ) -> TypeExpressionVisitResult<SpannedTypeError> {
        let members = intersection
            .0
            .iter_mut()
            .map(|member| self.infer_type_expression(member))
            .collect::<Result<Vec<_>, _>>()?;
        mark_type(Type::from(TypeDefinition::intersection(members)))
    }
    fn visit_structural_map_type(
        &mut self,
        structural_map: &mut StructuralMap,
        _: &Range<usize>,
    ) -> TypeExpressionVisitResult<SpannedTypeError> {
        let mut fields = vec![];
        for (field_name, field_type_expr) in structural_map.0.iter_mut() {
            let field_name = self.infer_type_expression(field_name)?;
            let field_type = self.infer_type_expression(field_type_expr)?;
            fields.push((field_name, field_type));
        }
        mark_type_definition(TypeDefinition::Map(fields.into_iter().collect()))
    }
    fn visit_structural_list_type(
        &mut self,
        structural_list: &mut StructuralList,
        _: &Range<usize>,
    ) -> TypeExpressionVisitResult<SpannedTypeError> {
        mark_type_definition(TypeDefinition::List(
            structural_list
                .0
                .iter_mut()
                .map(|elem_type_expr| {
                    self.infer_type_expression(elem_type_expr)
                })
                .collect::<Result<ListTypeDefinition, SpannedTypeError>>()?,
        ))
    }

    fn visit_get_reference_type(
        &mut self,
        pointer_address: &mut PointerAddress,
        span: &Range<usize>,
    ) -> TypeExpressionVisitResult<SpannedTypeError> {
        mark_type(self.infer_type_from_pointer_address(
            pointer_address,
            Some(span.clone()),
        )?)
    }
    fn visit_variable_access_type(
        &mut self,
        var_access: &mut VariableAccess,
        _: &Range<usize>,
    ) -> TypeExpressionVisitResult<SpannedTypeError> {
        mark_type_or_never(self.variable_type(var_access.id))
    }
    fn visit_fixed_size_list_type(
        &mut self,
        _fixed_size_list: &mut FixedSizeList,
        span: &Range<usize>,
    ) -> TypeExpressionVisitResult<SpannedTypeError> {
        Err(SpannedTypeError::new_with_span(
            TypeError::Unimplemented(
                "FixedSizeList type inference not implemented".into(),
            ),
            span.clone(),
        ))
    }

    fn visit_callable_type(
        &mut self,
        callable_type: &mut CallableTypeExpression,
        _: &Range<usize>,
    ) -> TypeExpressionVisitResult<SpannedTypeError> {
        let return_type = match &mut callable_type.return_type {
            Some(return_type) => Some(self.infer_type_expression(return_type)?),
            None => None,
        };

        let yeet_type = match &mut callable_type.yeet_type {
            Some(yeet_type) => Some(self.infer_type_expression(yeet_type)?),
            None => None,
        };

        let parameter_types = callable_type
            .parameter_types
            .iter_mut()
            .map(|(key, param_type_expr)| {
                let param_type = self.infer_type_expression(param_type_expr)?;
                Ok((key.clone(), param_type))
            })
            .collect::<Result<Vec<_>, SpannedTypeError>>()?;

        let rest_parameter_type = match &mut callable_type.rest_parameter_type {
            Some((key, rest_param_type_expr)) => {
                let rest_param_type =
                    self.infer_type_expression(rest_param_type_expr)?;
                Some((key.clone(), Box::new(rest_param_type)))
            }
            None => None,
        };

        mark_type(Type::from(TypeDefinition::Callable(
            CallableTypeDefinition {
                kind: callable_type.kind,
                requires_async: false, // TODO
                parameters: parameter_types,
                rest_parameter: rest_parameter_type,
                return_type: return_type.map(Box::new),
                yeet_type: yeet_type.map(Box::new),
            },
        )))
    }
    fn visit_generic_access_type(
        &mut self,
        _generic_access: &mut GenericAccess,
        span: &Range<usize>,
    ) -> TypeExpressionVisitResult<SpannedTypeError> {
        Err(SpannedTypeError::new_with_span(
            TypeError::Unimplemented(
                "GenericAccess type inference not implemented".into(),
            ),
            span.clone(),
        ))
    }
    fn visit_type_identifier(
        &mut self,
        _literal: &mut String,
        _span: &Range<usize>,
    ) -> TypeExpressionVisitResult<SpannedTypeError> {
        unreachable!(
            "Literal type expressions should have been resolved during precompilation"
        );
    }
    fn visit_ref_mut_type(
        &mut self,
        type_ref_mut: &mut TypeExpression,
        _span: &Range<usize>,
    ) -> TypeExpressionVisitResult<SpannedTypeError> {
        let inner_type = self.infer_type_expression(type_ref_mut)?;
        mark_type(inner_type)
    }
    fn visit_ref_type(
        &mut self,
        type_ref: &mut TypeExpression,
        _span: &Range<usize>,
    ) -> TypeExpressionVisitResult<SpannedTypeError> {
        let inner_type = self.infer_type_expression(type_ref)?;
        mark_type(inner_type)
    }
    fn visit_shared_type(
        &mut self,
        type_shared: &mut TypeExpression,
        _span: &Range<usize>,
    ) -> TypeExpressionVisitResult<SpannedTypeError> {
        let inner_type = self.infer_type_expression(type_shared)?;
        mark_type(inner_type)
    }
    fn visit_slice_list_type(
        &mut self,
        _slice_list: &mut SliceList,
        span: &Range<usize>,
    ) -> TypeExpressionVisitResult<SpannedTypeError> {
        Err(SpannedTypeError::new_with_span(
            TypeError::Unimplemented(
                "SliceList type inference not implemented".into(),
            ),
            span.clone(),
        ))
    }
    fn visit_variant_access_type(
        &mut self,
        _variant_access: &mut TypeVariantAccess,
        span: &Range<usize>,
    ) -> TypeExpressionVisitResult<SpannedTypeError> {
        Err(SpannedTypeError::new_with_span(
            TypeError::Unimplemented(
                "VariantAccess type inference not implemented".into(),
            ),
            span.clone(),
        ))
    }

    fn visit_get_core_lib_type(
        &mut self,
        core_lib_type_id: &mut CoreLibTypeId,
        _span: &Range<usize>,
    ) -> TypeExpressionVisitResult<SpannedTypeError> {
        mark_type(TypeDefinition::CoreType(*core_lib_type_id).into())
    }
}

impl<'a> TypeInference<'a> {
    fn infer_type_from_pointer_address(
        &mut self,
        pointer_address: &PointerAddress,
        span: Option<Range<usize>>,
    ) -> Result<Type, SpannedTypeError> {
        let ty = if let Some(container) =
            self.memory.get_reference(pointer_address)
        {
            let container = SharedContainer::Referenced(container);
            let value = container.collapsed_value();

            if let Some(CoreValue::Type(ty)) =
                &value.borrow().try_as_core_value()
            {
                Some(ty.clone())
            } else {
                None
            }
        } else {
            None
        };

        ty.ok_or(SpannedTypeError::new(
            TypeError::ReferenceToNonTypeValue,
            span,
        ))
    }
}

impl<'a> ExpressionVisitor<SpannedTypeError> for TypeInference<'a> {
    fn visit_get_ref(
        &mut self,
        create_ref: &mut DeriveRef,
        _: &Range<usize>,
    ) -> ExpressionVisitResult<SpannedTypeError> {
        let inner_type = self.infer_expression(&mut create_ref.expression)?;

        mark_type(inner_type.box_with_metadata(TypeMetadata::Local {
            mutability: LocalMutability::Immutable,
            ownership: LocalOwnership::Referenced(create_ref.mutability),
        }))
    }

    fn visit_create_shared(
        &mut self,
        create_shared: &mut CreateShared,
        _: &Range<usize>,
    ) -> ExpressionVisitResult<SpannedTypeError> {
        let inner_type =
            self.infer_expression(&mut create_shared.expression)?;

        mark_type(inner_type.box_with_metadata(TypeMetadata::Shared {
            mutability: create_shared.mutability,
            ownership: SharedContainerOwnership::Owned,
        }))
    }

    fn visit_get_shared_ref(
        &mut self,
        get_shared_ref: &mut DeriveSharedRef,
        span: &Range<usize>,
    ) -> ExpressionVisitResult<SpannedTypeError> {
        let inner_type =
            self.infer_expression(&mut get_shared_ref.expression)?;

        mark_type(
            inner_type
                .try_convert_to_shared_ref(get_shared_ref.mutability)
                .map_err(|_| {
                    SpannedTypeError::new_with_span(
                        TypeError::InvalidSharedReference,
                        span.clone(),
                    )
                })?,
        )
    }

    fn handle_expression_error(
        &mut self,
        error: SpannedTypeError,
        _: &DatexExpression,
    ) -> Result<VisitAction<DatexExpression>, SpannedTypeError> {
        self.record_error(error)
    }

    fn visit_statements(
        &mut self,
        statements: &mut Statements,
        _: &Range<usize>,
    ) -> ExpressionVisitResult<SpannedTypeError> {
        let mut inferred_type = Type::core(CoreLibBaseTypeId::Unit);

        // Infer type for each statement in order
        for statement in statements.statements.iter_mut() {
            inferred_type = self.infer_expression(statement)?;
        }

        // If the statements block ends with a terminator (semicolon, etc.),
        // it returns the unit type, otherwise, it returns the last inferred type.
        if statements.is_terminated {
            inferred_type = Type::core(CoreLibBaseTypeId::Unit);
        }

        Ok(VisitAction::SetTypeSkipChildren(inferred_type))
    }

    fn visit_variable_access(
        &mut self,
        var_access: &mut VariableAccess,
        _: &Range<usize>,
    ) -> ExpressionVisitResult<SpannedTypeError> {
        mark_type_or_never(self.variable_type(var_access.id))
    }

    fn visit_property_assignment(
        &mut self,
        property_assignment: &mut PropertyAssignment,
        _span: &Range<usize>,
    ) -> ExpressionVisitResult<SpannedTypeError> {
        let assigned_type = self
            .infer_expression(&mut property_assignment.assigned_expression)?;

        match property_assignment.operator {
            None => {}
            _ => {
                panic!("Unsupported assignment operator");
            }
        }
        mark_type(assigned_type)
    }

    fn visit_variable_assignment(
        &mut self,
        variable_assignment: &mut VariableAssignment,
        span: &Range<usize>,
    ) -> ExpressionVisitResult<SpannedTypeError> {
        let Some(id) = variable_assignment.id else {
            panic!(
                "VariableAssignment should have an id assigned during precompilation"
            );
        };

        let _var = self.variable_type(id).expect("Variable must be present");

        let assigned_type =
            self.infer_expression(&mut variable_assignment.expression)?;
        let annotated_type = self
            .variable_type(id)
            .unwrap_or_else(|| Type::core(CoreLibBaseTypeId::Never));

        match variable_assignment.operator {
            None => {
                if !annotated_type.is_superset_of(&assigned_type) {
                    return Err(SpannedTypeError::new_with_span(
                        TypeError::assignment_type_mismatch(
                            annotated_type,
                            assigned_type,
                        ),
                        span.clone(),
                    ));
                }
            }
            _ => {
                panic!("Unsupported assignment operator");
            }
        }
        mark_type(annotated_type)
    }

    fn visit_integer(
        &mut self,
        integer: &mut Integer,
        _: &Range<usize>,
    ) -> ExpressionVisitResult<SpannedTypeError> {
        mark_literal_type(LiteralTypeDefinition::Integer(integer.clone()))
    }
    fn visit_typed_integer(
        &mut self,
        typed_integer: &mut TypedInteger,
        _: &Range<usize>,
    ) -> ExpressionVisitResult<SpannedTypeError> {
        mark_literal_type(LiteralTypeDefinition::TypedInteger(
            typed_integer.clone(),
        ))
    }
    fn visit_decimal(
        &mut self,
        decimal: &mut Decimal,
        _: &Range<usize>,
    ) -> ExpressionVisitResult<SpannedTypeError> {
        mark_literal_type(LiteralTypeDefinition::Decimal(decimal.clone()))
    }
    fn visit_typed_decimal(
        &mut self,
        decimal: &mut TypedDecimal,
        _: &Range<usize>,
    ) -> ExpressionVisitResult<SpannedTypeError> {
        mark_literal_type(LiteralTypeDefinition::TypedDecimal(decimal.clone()))
    }
    fn visit_boolean(
        &mut self,
        boolean: &mut Boolean,
        _: &Range<usize>,
    ) -> ExpressionVisitResult<SpannedTypeError> {
        mark_literal_type(LiteralTypeDefinition::Boolean(boolean.clone()))
    }
    fn visit_text(
        &mut self,
        text: &mut Text,
        _: &Range<usize>,
    ) -> ExpressionVisitResult<SpannedTypeError> {
        mark_literal_type(LiteralTypeDefinition::Text(text.clone()))
    }
    fn visit_null(
        &mut self,
        _: &Range<usize>,
    ) -> ExpressionVisitResult<SpannedTypeError> {
        mark_type(Type::core(CoreLibBaseTypeId::Null))
    }
    fn visit_endpoint(
        &mut self,
        endpoint: &mut Endpoint,
        _: &Range<usize>,
    ) -> ExpressionVisitResult<SpannedTypeError> {
        mark_literal_type(LiteralTypeDefinition::Endpoint(endpoint.clone()))
    }
    fn visit_variable_declaration(
        &mut self,
        variable_declaration: &mut VariableDeclaration,
        span: &Range<usize>,
    ) -> ExpressionVisitResult<SpannedTypeError> {
        let init_type =
            self.infer_expression(&mut variable_declaration.init_expression)?;
        let actual_type =
            if let Some(specific) = &mut variable_declaration.type_annotation {
                // FIXME #619 check if matches
                let annotated_type = self.infer_type_expression(specific)?;
                if !init_type.is_subset_of(&annotated_type) {
                    self.record_error(SpannedTypeError::new_with_span(
                        TypeError::assignment_type_mismatch(
                            annotated_type.clone(),
                            init_type,
                        ),
                        span.clone(),
                    ))?;
                }
                annotated_type
            } else {
                init_type
            };
        self.update_variable_type(
            variable_declaration.id.unwrap(),
            actual_type.clone(),
        );
        mark_type(actual_type)
    }

    fn visit_binary_operation(
        &mut self,
        binary_operation: &mut BinaryOperation,
        span: &Range<usize>,
    ) -> ExpressionVisitResult<SpannedTypeError> {
        let left_type = self.infer_expression(&mut binary_operation.left)?;
        let right_type = self.infer_expression(&mut binary_operation.right)?;

        match binary_operation.operator {
            BinaryOperator::Arithmetic(op) => {
                // if base types are the same, use that as result type
                let ty = left_type.with_collapsed_type_definition(|left_def| {
                    right_type.with_collapsed_type_definition(
                        |right_def| match (left_def, right_def) {
                            (
                                TypeDefinition::Literal(_),
                                TypeDefinition::Literal(_),
                            ) => {
                                if left_def.base_core_lib_type()
                                    == right_def.base_core_lib_type()
                                {
                                    Some(Type::core(
                                        left_type.base_core_lib_type(),
                                    ))
                                } else {
                                    None
                                }
                            }
                            _ => None,
                        },
                    )
                });

                if let Some(ty) = ty {
                    mark_type(ty)
                } else {
                    Err(SpannedTypeError::new_with_span(
                        TypeError::mismatched_operands(
                            op, left_type, right_type,
                        ),
                        span.clone(),
                    ))
                }
            }
            _ => {
                //  otherwise, use never type
                self.record_error(SpannedTypeError::new_with_span(
                    TypeError::Unimplemented(
                        "Binary operation not implemented".into(),
                    ),
                    span.clone(),
                ))?;
                mark_never()
            }
        }
    }

    fn visit_type_declaration(
        &mut self,
        type_declaration: &mut TypeDeclarationExpression,
        _: &Range<usize>,
    ) -> ExpressionVisitResult<SpannedTypeError> {
        let type_id = type_declaration.id.expect(
            "TypeDeclarationExpression should have an id assigned during precompilation",
        );
        let var_type = self.variable_type(type_id);
        let _type_def = var_type.as_ref().expect(
            "TypeDeclarationExpression type should have been inferred already",
        );
        let inferred_type_def =
            self.infer_type_expression(&mut type_declaration.definition)?;

        self.update_variable_type(type_id, inferred_type_def.clone());
        mark_type(inferred_type_def.clone())
    }

    fn visit_entity_declaration(
        &mut self,
        entity_declaration: &mut EntityDeclarationExpression,
        _span: &Range<usize>,
    ) -> ExpressionVisitResult<SpannedTypeError> {
        let type_id = entity_declaration.id.expect(
            "EntityDeclarationExpression should have an id assigned during precompilation",
        );
        let var_type = self.variable_type(type_id);
        let type_def = var_type
            .as_ref()
            .expect("EntityDeclarationExpression type should have been inferred already");
        let inferred_type_def =
            self.infer_type_expression(&mut entity_declaration.definition)?;

        match &type_def {
            Type::Entity(definition) => {
                let mut val = definition.collapsed_value_mut();
                match &mut val.borrow_mut().try_as_core_value_mut() {
                    Some(CoreValue::EntityTypeDefinition(nominal_def)) => {
                        nominal_def.replace_definition(
                            inferred_type_def.convert_to_definition(),
                        );
                    }
                    _ => {
                        panic!(
                            "Expected nominal type to be an alias during type declaration inference"
                        )
                    }
                }
            }
            Type::Definition(_r) => {
                // FIXME #620 is this necessary?
                // reference.borrow_mut().type_value = Type::new(
                //     TypeDefinition::Shared(r.clone()),
                //     TypeMetadata::default(),
                // );
                unreachable!(
                    "Type aliases should have been resolved during precompilation"
                );
                // r.definition = TypeDefinition::Shared(SharedContainerContainingType::new_unchecked(
                //     SharedContainer::
                // ));
            }
        }
        mark_type(type_def.clone())
    }

    fn visit_list(
        &mut self,
        list: &mut List,
        _: &Range<usize>,
    ) -> ExpressionVisitResult<SpannedTypeError> {
        mark_type_definition(TypeDefinition::List(
            list.items
                .iter_mut()
                .map(|elem_type_expr| self.infer_expression(elem_type_expr))
                .collect::<Result<ListTypeDefinition, _>>()?,
        ))
    }

    fn visit_range(
        &mut self,
        range: &mut RangeDeclaration,
        _span: &Range<usize>,
    ) -> ExpressionVisitResult<SpannedTypeError> {
        let x = self.infer_expression(&mut range.start)?;
        let y = self.infer_expression(&mut range.end)?;
        let z = TypeDefinition::Range(RangeTypeDefinition {
            start: Box::new(x),
            end: Box::new(y),
        });
        mark_type_definition(z)
    }

    fn visit_map(
        &mut self,
        map: &mut Map,
        _: &Range<usize>,
    ) -> ExpressionVisitResult<SpannedTypeError> {
        let mut fields = vec![];
        for (key_expr, value_expr) in map.entries.iter_mut() {
            let key_type = self.infer_expression(key_expr)?;
            let value_type = self.infer_expression(value_expr)?;
            fields.push((key_type, value_type));
        }
        mark_type_definition(TypeDefinition::Map(fields.into_iter().collect()))
    }

    fn visit_apply(
        &mut self,
        apply: &mut Apply,
        span: &Range<usize>,
    ) -> ExpressionVisitResult<SpannedTypeError> {
        let caller = self.infer_expression(&mut apply.base)?;
        caller.with_collapsed_type_definition(|ty| {
            match ty {
                TypeDefinition::Callable(callable_signature) => {
                    // FIXME handle signature mismatch errors
                    // FIXME handle yeet type and mark error type in returned type
                    if let Some(return_type) =
                        callable_signature.return_type.as_ref()
                    {
                        mark_type(*return_type.clone())
                    } else {
                        mark_type(Type::core(CoreLibBaseTypeId::Never))
                    }
                }
                _ => Err(SpannedTypeError::new_with_span(
                    TypeError::unsupported_apply(caller.clone()),
                    span.clone(),
                )),
            }
        })
    }

    fn visit_property_access(
        &mut self,
        _property_access: &mut PropertyAccess,
        span: &Range<usize>,
    ) -> ExpressionVisitResult<SpannedTypeError> {
        let base = self.infer_expression(&mut _property_access.base)?;
        let property = self.infer_expression(&mut _property_access.property)?;
        base.with_collapsed_type_definition(|d| match d {
            // TODO handle structural access, add null to union for dynamic maps
            TypeDefinition::Map(map) => mark_type(Type::Definition(
                TypeDefinition::union(
                    map.iter().map(|(_, v)| v.clone()).collect::<Vec<_>>(),
                )
                .into(),
            )),
            TypeDefinition::List(members) => {
                if !property.with_collapsed_type_definition(|d| {
                    matches!(
                        d,
                        TypeDefinition::Literal(
                            LiteralTypeDefinition::Integer(_)
                        ) | TypeDefinition::Literal(
                            LiteralTypeDefinition::TypedInteger(_)
                        )
                    )
                }) {
                    return Err(SpannedTypeError::new_with_span(
                        TypeError::unsupported_property_access(
                            base.clone(),
                            property.clone(),
                        ),
                        span.clone(),
                    ));
                }
                // FIXME handle out of bounds access for structural lists and infer correct type at index
                // handle union null case for non-structural lists
                mark_type(Type::Definition(
                    TypeDefinition::union(members.to_vec()).into(),
                ))
            }
            _ => Err(SpannedTypeError::new_with_span(
                TypeError::unsupported_property_access(
                    base.clone(),
                    property.clone(),
                ),
                span.clone(),
            )),
        })
    }

    fn visit_generic_instantiation(
        &mut self,
        _generic_instantiation: &mut GenericInstantiation,
        span: &Range<usize>,
    ) -> ExpressionVisitResult<SpannedTypeError> {
        Err(SpannedTypeError::new_with_span(
            TypeError::Unimplemented(
                "GenericInstantiation type inference not implemented".into(),
            ),
            span.clone(),
        ))
    }

    fn visit_comparison_operation(
        &mut self,
        _comparison_operation: &mut ComparisonOperation,
        _span: &Range<usize>,
    ) -> ExpressionVisitResult<SpannedTypeError> {
        mark_type(Type::core(CoreLibBaseTypeId::Boolean))
    }
    fn visit_conditional(
        &mut self,
        conditional: &mut Conditional,
        _span: &Range<usize>,
    ) -> ExpressionVisitResult<SpannedTypeError> {
        self.infer_expression(&mut conditional.condition)?;
        let then_type = self.infer_expression(&mut conditional.then_branch)?;
        if let Some(else_branch) = &mut conditional.else_branch {
            self.infer_expression(else_branch)?;
        }
        mark_type(then_type)
    }

    fn visit_unbox(
        &mut self,
        unbox: &mut Unbox,
        span: &Range<usize>,
    ) -> ExpressionVisitResult<SpannedTypeError> {
        let inner_type = self.infer_expression(&mut unbox.expression)?;
        // remove most outer &/' if applicable
        let unbox_type = if let Type::Definition(definition) = inner_type {
            match definition.metadata {
                // non-unboxable local value
                TypeMetadata::Local { .. } => {
                    self.record_error(SpannedTypeError::new_with_span(
                        TypeError::invalid_unbox_type(Type::Definition(
                            definition,
                        )),
                        span.clone(),
                    ))?;
                    Type::core(CoreLibBaseTypeId::Never)
                }
                // *(shared 'shared X) -> 'shared X
                // shared (X) -> 23
                _ => {
                    match definition.definition {
                        // if nested type, collapse
                        TypeDefinition::Box(ty) => *ty,
                        // else, just remove ref
                        def => {
                            Type::Definition(TypeDefinitionWithMetadata::new(
                                def,
                                TypeMetadata::default(),
                            ))
                        }
                    }
                }
            }
        } else {
            self.record_error(SpannedTypeError::new_with_span(
                TypeError::invalid_unbox_type(inner_type),
                span.clone(),
            ))?;
            Type::core(CoreLibBaseTypeId::Never)
        };

        // check if type is actually unboxable (must be a shared container, TODO: maybe also copyable values)
        match unbox_type {
            Type::Definition(TypeDefinitionWithMetadata {
                metadata: TypeMetadata::Shared { .. },
                ..
            }) => mark_type(unbox_type),
            _ => {
                self.record_error(SpannedTypeError::new_with_span(
                    TypeError::invalid_unbox_type(unbox_type),
                    span.clone(),
                ))?;
                mark_never()
            }
        }
    }

    fn visit_callable_declaration(
        &mut self,
        callable_declaration: &mut CallableDeclaration,
        span: &Range<usize>,
    ) -> ExpressionVisitResult<SpannedTypeError> {
        let annotated_return_type = if let Some(return_type) =
            &mut callable_declaration.signature.return_type
        {
            Some(Box::new(self.infer_type_expression(return_type)?))
        } else {
            None
        };

        let annotated_yeet_type = if let Some(yeet_type) =
            &mut callable_declaration.signature.yeet_type
        {
            Some(Box::new(self.infer_type_expression(yeet_type)?))
        } else {
            None
        };

        let inferred_return_type = self
            .infer_expression(&mut callable_declaration.body)
            .unwrap_or_else(|_| Type::core(CoreLibBaseTypeId::Never));

        let rest_parameter_type = if let Some((name, rest_param)) =
            &mut callable_declaration.signature.rest_parameter
        {
            Some((
                Some(name.clone()),
                Box::new(self.infer_type_expression(rest_param)?),
            ))
        } else {
            None
        };

        let parameters = callable_declaration
            .signature
            .parameters
            .iter_mut()
            .map(|(name, param_type_expr)| {
                let param_type = self
                    .infer_type_expression(param_type_expr)
                    .unwrap_or_else(|_| Type::core(CoreLibBaseTypeId::Never));
                (Some(name.clone()), param_type)
            })
            .collect();

        let signature = CallableTypeDefinition {
            kind: callable_declaration.signature.kind,
            requires_async: false, // TODO
            parameters,
            rest_parameter: rest_parameter_type,
            return_type: annotated_return_type,
            yeet_type: annotated_yeet_type,
        };

        // Check if inferred return type matches the annotated return type
        // if an annotated return type is provided
        // If they don't match, record an error
        // TODO #622: improve
        if let Some(annotated_return_type) = &signature.return_type
            && !inferred_return_type
                .is_subset_of(annotated_return_type.as_ref())
        {
            self.record_error(SpannedTypeError::new_with_span(
                TypeError::assignment_type_mismatch(
                    *annotated_return_type.clone(),
                    inferred_return_type,
                ),
                span.clone(),
            ))?;
        }

        // Use the annotated type despite the mismatch
        mark_type(Type::from(TypeDefinition::Callable(signature)))
    }

    fn visit_unary_operation(
        &mut self,
        unary_operation: &mut UnaryOperation,
        span: &Range<usize>,
    ) -> ExpressionVisitResult<SpannedTypeError> {
        let op = unary_operation.operator;
        let inner = self.infer_expression(&mut unary_operation.expression)?;
        mark_type(match op {
            UnaryOperator::Logical(op) => match op {
                LogicalUnaryOperator::Not => {
                    Type::core(CoreLibBaseTypeId::Boolean)
                }
            },
            UnaryOperator::Arithmetic(_) | UnaryOperator::Bitwise(_) => inner
                .with_collapsed_type_definition(|ty| Type::from(ty.clone())),
            UnaryOperator::Reference(_) => {
                return Err(SpannedTypeError::new_with_span(
                    TypeError::Unimplemented(
                        "Unary reference operator type inference not implemented"
                            .into(),
                    ),
                    span.clone(),
                ));
            }
        })
    }
    fn visit_variant_access(
        &mut self,
        variant_access: &mut VariantAccess,
        span: &Range<usize>,
    ) -> ExpressionVisitResult<SpannedTypeError> {
        fn variant_type_id_from_pointer_address(
            _pointer_address: &PointerAddress,
            _variant_access: &VariantAccess,
            span: &Range<usize>,
        ) -> ExpressionVisitResult<SpannedTypeError> {
            // TODO implement variant access
            // Ok(VisitAction::ReplaceRecurse(DatexExpression::new(
            //         DatexExpressionData::RequestSharedRef(RequestSharedRef {
            //             address,
            //             mutability: ReferenceMutability::Immutable,
            //         }),
            //         span.clone(),
            //     )))

            Err(SpannedTypeError::new_with_span(
                TypeError::Unimplemented(
                    "VariantAccess is not implemented yet".into(),
                ),
                span.clone(),
            ))
        }

        fn variant_type_id(
            core_lib_id: CoreLibId,
            variant_access: &VariantAccess,
            span: &Range<usize>,
        ) -> ExpressionVisitResult<SpannedTypeError> {
            let variant_id = match core_lib_id {
                CoreLibId::Type(CoreLibTypeId::Base(base_id)) => {
                    base_id.variant(&variant_access.variant)
                }
                _ => {
                    return Err(SpannedTypeError::new_with_span(
                        TypeError::Unimplemented(
                            "Invalid core base type".into(),
                        ),
                        span.clone(),
                    ));
                }
            }
            .map_err(|_| {
                SpannedTypeError::new_with_span(
                    TypeError::subvariant_not_found(
                        variant_access.name.clone(),
                        variant_access.variant.clone(),
                    ),
                    span.clone(),
                )
            })?;

            Ok(VisitAction::ReplaceRecurse(DatexExpression::new(
                DatexExpressionData::ResolveCoreLibId(CoreLibId::Type(
                    variant_id.into(),
                )),
                span.clone(),
            )))
        }

        match &variant_access.base {
            // Handle variant access on a variable
            ResolvedVariable::VariableId(id) => {
                // we expect the variable to be of TypeReference type
                let base_type = self.variable_type(*id).ok_or(
                    SpannedTypeError::new_with_span(
                        TypeError::Unimplemented(
                            "VariantAccess base variable type not found".into(),
                        ),
                        span.clone(),
                    ),
                )?;

                // if it's a Type::Nominal, and it has the pointer address set, we can
                // remap the expression to a GetReference
                match base_type {
                    Type::Entity(reference) => {
                        variant_type_id_from_pointer_address(
                            &reference.pointer_address(),
                            variant_access,
                            span,
                        )
                    }
                    Type::Definition(alias) => {
                        match &alias.definition {
                            TypeDefinition::CoreType(core_lib_id) => {
                                variant_type_id(CoreLibId::Type(*core_lib_id), variant_access, span)
                            }
                            _ => {
                                Err(SpannedTypeError::new_with_span(
                                    TypeError::Unimplemented(
                                        "VariantAccess on non-nominal type alias not implemented".into(),
                                    ),
                                    span.clone(),
                                ))
                            }
                        }
                    }
                }
            }

            ResolvedVariable::PointerAddress(addr) => {
                variant_type_id_from_pointer_address(addr, variant_access, span)
            }
            ResolvedVariable::CoreLibId(core_lib_id) => {
                variant_type_id(*core_lib_id, variant_access, span)
            }
        }
    }

    fn visit_get_core_lib_id(
        &mut self,
        core_lib_id: &mut CoreLibId,
        span: &Range<usize>,
    ) -> ExpressionVisitResult<SpannedTypeError> {
        match core_lib_id {
            CoreLibId::Type(type_id) => mark_type(TypeDefinition::CoreType(
                *type_id
            )
            .into()),
            _ => Err(SpannedTypeError::new_with_span(
                TypeError::Unimplemented(
                    "Only CoreLibId::Type is supported in get_core_lib_id expressions for now".into(),
                ),
                span.clone(),
            )),
        }
    }

    fn visit_stack_index(
        &mut self,
        _stack_index: &StackIndex,
        span: &Range<usize>,
    ) -> ExpressionVisitResult<SpannedTypeError> {
        Err(SpannedTypeError::new_with_span(
            TypeError::Unimplemented(
                "Stack index inference not implemented".into(),
            ),
            span.clone(),
        ))
    }
    fn visit_identifier(
        &mut self,
        _identifier: &mut String,
        _span: &Range<usize>,
    ) -> ExpressionVisitResult<SpannedTypeError> {
        Ok(VisitAction::AbortRecursion)
    }
    fn visit_placeholder(
        &mut self,
        _placeholder_type: &mut ValueAccessType,
        _span: &Range<usize>,
    ) -> ExpressionVisitResult<SpannedTypeError> {
        Ok(VisitAction::AbortRecursion)
    }
    fn visit_unbox_assignment(
        &mut self,
        unbox_assignment: &mut UnboxAssignment,
        span: &Range<usize>,
    ) -> ExpressionVisitResult<SpannedTypeError> {
        // FIXME #623: handle type checking and if unbox assignment is valid
        let expression_type =
            self.infer_expression(&mut unbox_assignment.unbox_expression)?;

        let inner_type = expression_type.as_definition_with_metadata(|e| {
            let ownership = e.metadata.shared_container_ownership();
            let _mutability = e.metadata.shared_mutability();

            if ownership
                != Some(&SharedContainerOwnership::Referenced(
                    ReferenceMutability::Mutable,
                ))
                && ownership != Some(&SharedContainerOwnership::Owned)
            {
                return Err(SpannedTypeError::new_with_span(
                    TypeError::AssignmentToImmutableReference("".to_string()),
                    span.clone(),
                ));
            }
            match &e.definition {
                TypeDefinition::Box(ty) => Ok(*ty.clone()),
                TypeDefinition::Shared(sh) => {
                    Ok(sh.with_collapsed_type_value(|ty| ty.clone()))
                }
                _ => Err(SpannedTypeError::new_with_span(
                    TypeError::invalid_unbox_type(expression_type.clone()),
                    span.clone(),
                )),
            }
        })?;

        let assigned_type =
            self.infer_expression(&mut unbox_assignment.assigned_expression)?;

        // FIXME #624 implement proper type matching
        if !assigned_type.is_subset_of(&inner_type) {
            return Err(SpannedTypeError::new_with_span(
                TypeError::assignment_type_mismatch(inner_type, assigned_type),
                span.clone(),
            ));
        }

        mark_type(assigned_type)
    }

    fn visit_request_shared_reference(
        &mut self,
        shared_ref: &mut RequestSharedRef,
        span: &Range<usize>,
    ) -> ExpressionVisitResult<SpannedTypeError> {
        // FIXME handle mutability
        mark_type(self.infer_type_from_pointer_address(
            &shared_ref.address,
            Some(span.clone()),
        )?)
    }

    fn visit_stack_assignment(
        &mut self,
        _slot_assignment: &mut StackAssignment,
        span: &Range<usize>,
    ) -> ExpressionVisitResult<SpannedTypeError> {
        Err(SpannedTypeError::new_with_span(
            TypeError::Unimplemented(
                "SlotAssignment type inference not implemented".into(),
            ),
            span.clone(),
        ))
    }
    fn visit_remote_execution(
        &mut self,
        _remote_execution: &mut RemoteExecution,
        span: &Range<usize>,
    ) -> ExpressionVisitResult<SpannedTypeError> {
        Err(SpannedTypeError::new_with_span(
            TypeError::Unimplemented(
                "RemoteExecution type inference not implemented".into(),
            ),
            span.clone(),
        ))
    }
}

#[cfg(test)]
#[allow(clippy::std_instead_of_core, clippy::std_instead_of_alloc)]
mod tests {
    use crate::{
        ast::{
            expressions::{
                BinaryOperation, DatexExpression, DatexExpressionData, List,
                Map, VariableDeclaration, VariableKind,
            },
            spanned::Spanned,
        },
        compiler::precompiler::{
            precompile_ast_simple_error,
            precompiled_ast::{AstMetadata, RichAst},
            scope_stack::PrecompilerScopeStack,
        },
        global::operators::{BinaryOperator, binary::ArithmeticOperator},
        libs::core::type_id::{
            CoreLibBaseTypeId, CoreLibTypeId, CoreLibVariantTypeId,
        },
        parser::Parser,
        prelude::*,
        runtime::{
            Runtime, cache::shared_references_cache::SharedReferencesCache,
            pointer_address_provider::SelfOwnedPointerAddressProvider,
        },
        shared_values::{
            ReferenceMutability, SharedContainerMutability,
            SharedContainerOwnership,
        },
        type_inference::{
            InferOutcome,
            error::{SimpleOrDetailedTypeError, SpannedTypeError},
            infer_expression_type_detailed_errors,
            infer_expression_type_simple_error,
            infer_expression_type_with_errors,
        },
        types::{
            entities::entity_type_definition::EntityTypeDefinition,
            entity_type::EntityType,
            error::TypeError,
            literal_type_definition::LiteralTypeDefinition,
            shared_container_containing_type::SharedContainerContainingType,
            r#type::Type,
            type_definition::{
                TypeDefinition,
                callable::{CallableKind, CallableTypeDefinition},
                intersection::IntersectionTypeDefinition,
                union::UnionTypeDefinition,
            },
            type_definition_with_metadata::{
                TypeDefinitionWithMetadata, TypeMetadata,
            },
        },
        values::{
            core_value::CoreValue,
            core_values::{
                boolean::Boolean,
                decimal::{Decimal, typed_decimal::TypedDecimal},
                endpoint::Endpoint,
                integer::{
                    Integer,
                    typed_integer::{IntegerTypeVariant, TypedInteger},
                },
            },
        },
    };
    use core::{assert_matches, cell::RefCell, ops::Deref, str::FromStr};

    /// Infers type errors for the given source code.
    /// Panics if parsing or precompilation succeeds.
    fn errors_for_script(src: &str) -> Vec<SpannedTypeError> {
        let runtime = Runtime::stub();
        let ast = Parser::parse_with_default_options(src).unwrap();
        let mut scope_stack = PrecompilerScopeStack::default();
        let ast_metadata = Rc::new(RefCell::new(AstMetadata::default()));
        let mut res = precompile_ast_simple_error(
            ast,
            &mut scope_stack,
            ast_metadata,
            runtime.clone(),
        )
        .expect("Precompilation failed");
        infer_expression_type_detailed_errors(
            &mut res,
            &mut *runtime.shared_references_cache_refcell().borrow_mut(),
        )
        .expect_err("Expected type errors")
        .errors
    }

    /// Infers type errors for the given expression.
    /// Panics if precompilation succeeds.
    fn errors_for_expression(
        expr: &mut DatexExpression,
    ) -> Vec<SpannedTypeError> {
        let runtime = Runtime::stub();
        let mut scope_stack = PrecompilerScopeStack::default();
        let ast_metadata = Rc::new(RefCell::new(AstMetadata::default()));
        let mut rich_ast = precompile_ast_simple_error(
            expr.clone(),
            &mut scope_stack,
            ast_metadata,
            runtime.clone(),
        )
        .expect("Precompilation failed");
        infer_expression_type_detailed_errors(
            &mut rich_ast,
            &mut *runtime.shared_references_cache_refcell().borrow_mut(),
        )
        .expect_err("Expected type errors")
        .errors
    }

    /// Infers the AST of the given source code.
    /// Panics if parsing, precompilation or type inference fails.
    /// Returns the RichAst containing the inferred types.
    fn ast_for_script(src: &str) -> RichAst {
        let runtime = Runtime::stub();
        let ast = Parser::parse_with_default_options(src).unwrap();
        let mut scope_stack = PrecompilerScopeStack::default();
        let ast_metadata = Rc::new(RefCell::new(AstMetadata::default()));
        let mut res = precompile_ast_simple_error(
            ast,
            &mut scope_stack,
            ast_metadata,
            runtime.clone(),
        )
        .expect("Precompilation failed");

        if let Err(err) = infer_expression_type_simple_error(
            &mut res,
            &runtime.shared_references_cache_refcell().borrow(),
        ) {
            panic!("Type inference failed: {:#?}", err);
        } else {
            res
        }
    }

    /// Infers the AST of the given expression.
    /// Panics if type inference fails.
    fn ast_for_expression(expr: &mut DatexExpression) -> RichAst {
        let runtime = Runtime::stub();
        let mut scope_stack = PrecompilerScopeStack::default();
        let ast_metadata = Rc::new(RefCell::new(AstMetadata::default()));
        let mut rich_ast = precompile_ast_simple_error(
            expr.clone(),
            &mut scope_stack,
            ast_metadata,
            runtime.clone(),
        )
        .expect("Precompilation failed");
        infer_expression_type_simple_error(
            &mut rich_ast,
            &runtime.shared_references_cache_refcell().borrow(),
        )
        .expect("Type inference failed");
        rich_ast
    }

    /// Infers the type of the given source code.
    /// Panics if parsing, precompilation. Type errors are collected and ignored.
    /// Returns the inferred type of the full script expression. For example,
    /// for "var x = 42; x", it returns the type of "x", as this is the last expression of the statements.
    /// For "var x = 42;", it returns the never type, as the statement is terminated.
    /// For "10 + 32", it returns the type of the binary operation.
    fn infer_type_from_script_ignore_errors(src: &str) -> Type {
        infer_from_script(src).to_type()
    }

    fn infer_from_script(src: &str) -> InferOutcome {
        let runtime = Runtime::stub();
        let ast = Parser::parse_with_default_options(src).unwrap();
        let mut scope_stack = PrecompilerScopeStack::default();
        let ast_metadata = Rc::new(RefCell::new(AstMetadata::default()));
        let mut res = precompile_ast_simple_error(
            ast,
            &mut scope_stack,
            ast_metadata,
            runtime.clone(),
        )
        .expect("Precompilation failed");
        infer_expression_type_with_errors(
            &mut res,
            &runtime.shared_references_cache_refcell().borrow(),
        )
    }

    /// Infers the type of the given expression.
    /// Panics if type inference fails.
    fn infer_from_expression(expr: &mut DatexExpression) -> Type {
        let runtime = Runtime::stub();
        let mut scope_stack = PrecompilerScopeStack::default();
        let ast_metadata = Rc::new(RefCell::new(AstMetadata::default()));

        let mut rich_ast = precompile_ast_simple_error(
            expr.clone(),
            &mut scope_stack,
            ast_metadata,
            runtime.clone(),
        )
        .expect("Precompilation failed");
        infer_expression_type_simple_error(
            &mut rich_ast,
            &runtime.shared_references_cache_refcell().borrow(),
        )
        .expect("Type inference failed")
    }

    #[test]
    fn variant_access() {
        // variant access on type (inline)
        let src = r#"
        var x = integer/u8
        "#;
        let res = infer_type_from_script_ignore_errors(src);
        assert_eq!(
            res,
            Type::core(CoreLibVariantTypeId::Integer(IntegerTypeVariant::U8))
        );

        // variant access on type (separate)
        let src = r#"
        var x = integer;
        x/u8
        "#;
        let res = infer_type_from_script_ignore_errors(src);
        assert_eq!(
            res,
            Type::core(CoreLibVariantTypeId::Integer(IntegerTypeVariant::U8))
        );

        // variant access on type alias (inline)
        let src = r#"
        type x = integer/u8
        "#;
        let res = infer_type_from_script_ignore_errors(src);
        assert_eq!(
            res,
            Type::core(CoreLibVariantTypeId::Integer(IntegerTypeVariant::U8))
        );

        // variant access on type alias (separate)
        let src = r#"
        type x = integer;
        x/u8
        "#;
        let res = infer_type_from_script_ignore_errors(src);
        assert_eq!(
            res,
            Type::core(CoreLibVariantTypeId::Integer(IntegerTypeVariant::U8))
        );

        // invalid variant access on type alias
        let src = r#"
        type x = integer;
        x/whatever
        "#;
        let res = errors_for_script(src);
        assert_eq!(
            *res.get(0).unwrap().error,
            TypeError::SubvariantNotFound("x".into(), "whatever".into())
        );
    }

    #[test]
    fn infer_function_types() {
        let src = r#"
        function add(a: integer, b: integer) -> integer (
            42
        )
        "#;

        let res = infer_type_from_script_ignore_errors(src);
        assert_eq!(
            res,
            Type::from(TypeDefinition::Callable(CallableTypeDefinition {
                kind: CallableKind::Function,
                requires_async: false,
                parameters: vec![
                    (
                        Some("a".to_string()),
                        Type::core(CoreLibBaseTypeId::Integer),
                    ),
                    (
                        Some("b".to_string()),
                        Type::core(CoreLibBaseTypeId::Integer)
                    ),
                ],
                rest_parameter: None,
                return_type: Some(Box::new(Type::core(
                    CoreLibBaseTypeId::Integer
                ))),
                yeet_type: None,
            },))
        );

        let src = r#"
        function add(a: integer, b: integer) (
            42
        )
        "#;

        let res = infer_type_from_script_ignore_errors(src);
        assert_eq!(
            res,
            Type::from(TypeDefinition::Callable(CallableTypeDefinition {
                kind: CallableKind::Function,
                requires_async: false,
                parameters: vec![
                    (
                        Some("a".to_string()),
                        Type::core(CoreLibBaseTypeId::Integer)
                    ),
                    (
                        Some("b".to_string()),
                        Type::core(CoreLibBaseTypeId::Integer)
                    ),
                ],
                rest_parameter: None,
                return_type: None,
                yeet_type: None,
            },))
        );
    }

    #[test]
    fn infer_literal_types() {
        assert_eq!(
            infer_from_expression(
                &mut DatexExpressionData::Boolean(true.into())
                    .with_default_span()
            ),
            Type::from(LiteralTypeDefinition::Boolean(true.into()),)
        );

        assert_eq!(
            infer_from_expression(
                &mut DatexExpressionData::Boolean(false.into())
                    .with_default_span()
            ),
            Type::from(LiteralTypeDefinition::Boolean(false.into()),)
        );

        assert_eq!(
            infer_from_expression(
                &mut DatexExpressionData::Decimal(Decimal::from(1.23))
                    .with_default_span()
            ),
            Type::from(LiteralTypeDefinition::Decimal(Decimal::from(1.23)),)
        );

        assert_eq!(
            infer_from_expression(
                &mut DatexExpressionData::Integer(Integer::from(42))
                    .with_default_span()
            ),
            Type::from(LiteralTypeDefinition::Integer(Integer::from(42)),)
        );
        assert_eq!(
            infer_from_expression(
                &mut DatexExpressionData::List(List::new(vec![
                    DatexExpressionData::Integer(Integer::from(1))
                        .with_default_span(),
                    DatexExpressionData::Integer(Integer::from(2))
                        .with_default_span(),
                    DatexExpressionData::Integer(Integer::from(3))
                        .with_default_span()
                ]))
                .with_default_span()
            ),
            Type::Definition(
                TypeDefinition::List(
                    vec![
                        Type::from(LiteralTypeDefinition::Integer(
                            Integer::from(1)
                        )),
                        Type::from(LiteralTypeDefinition::Integer(
                            Integer::from(2)
                        )),
                        Type::from(LiteralTypeDefinition::Integer(
                            Integer::from(3)
                        ))
                    ]
                    .into_iter()
                    .collect()
                )
                .into()
            )
        );

        assert_eq!(
            infer_from_expression(
                &mut DatexExpressionData::Map(Map::new(vec![(
                    DatexExpressionData::Text("a".into()).with_default_span(),
                    DatexExpressionData::Integer(Integer::from(1))
                        .with_default_span()
                )]))
                .with_default_span()
            ),
            Type::Definition(
                TypeDefinition::Map(
                    vec![(
                        Type::Definition(
                            LiteralTypeDefinition::Text("a".into()).into()
                        ),
                        Type::Definition(
                            LiteralTypeDefinition::Integer(Integer::from(1))
                                .into()
                        )
                    )]
                    .into_iter()
                    .collect()
                )
                .into()
            )
        );
    }

    #[test]
    fn nominal_type_declaration() {
        let src = r#"
        entity A = integer;
        "#;
        let metadata = ast_for_script(src).metadata;
        let metadata = metadata.borrow();
        let var_a = metadata.variable_metadata(0).unwrap();

        if let Some(Type::Entity(container)) = &var_a.var_type {
            let def = container.entity_definition();
            assert_eq!(&def.name, "A");
            assert_eq!(
                &def.definition,
                &TypeDefinition::core(CoreLibBaseTypeId::Integer)
            );
        } else {
            panic!("expected nominal type");
        }
    }

    #[test]
    fn structural_type_declaration() {
        let src = r#"
        type A = integer;
        "#;
        let metadata = ast_for_script(src).metadata;
        let metadata = metadata.borrow();
        let var_a = metadata.variable_metadata(0).unwrap();
        let var_type = var_a.var_type.as_ref().unwrap();

        assert_matches!(
            var_type,
            Type::Definition(TypeDefinitionWithMetadata {
                definition: TypeDefinition::CoreType(CoreLibTypeId::Base(
                    CoreLibBaseTypeId::Integer
                )),
                ..
            })
        );

        let inferred_type =
            infer_type_from_script_ignore_errors("type X = integer/u8");
        assert_eq!(
            inferred_type,
            Type::core(CoreLibVariantTypeId::Integer(IntegerTypeVariant::U8))
        );

        let inferred_type =
            infer_type_from_script_ignore_errors("type X = decimal");
        assert_eq!(inferred_type, Type::core(CoreLibBaseTypeId::Decimal));

        let inferred_type =
            infer_type_from_script_ignore_errors("type X = boolean");
        assert_eq!(inferred_type, Type::core(CoreLibBaseTypeId::Boolean));

        let inferred_type =
            infer_type_from_script_ignore_errors("type X = text");
        assert_eq!(inferred_type, Type::core(CoreLibBaseTypeId::Text));
    }

    #[test]
    fn recursive_types() {
        // TODO:
        // let src = r#"
        // type A = { b: B };
        // type B = { a: A };
        // "#;
        // let metadata = ast_for_script(src).metadata;
        // let metadata = metadata.borrow();
        // let var = metadata.variable_metadata(0).unwrap();
        // let var_type = var.var_type.as_ref().unwrap();
        // assert_matches!(var_type.definition().structural_definition, TypeDefinition::Shared(_));
    }

    #[test]
    fn recursive_nominal_type() {
        let src = r#"
        entity LinkedList = {
            value: text,
            next: LinkedList | null
        };
        "#;
        let metadata = ast_for_script(src).metadata;
        let metadata = metadata.borrow();
        let var = metadata.variable_metadata(0).unwrap();
        let var_type = var.var_type.as_ref().unwrap();
        assert_matches!(var_type, Type::Entity(_));

        // get next field, as wrapped in union
        assert_eq!(
            var_type.with_collapsed_type_definition(|d| match d {
                TypeDefinition::Map(fields) => fields[1]
                    .1
                    .with_collapsed_type_definition(|inner| match inner {
                        TypeDefinition::Union(members) => {
                            assert_eq!(members.len(), 2);
                            members[0].clone()
                        }
                        _ => unreachable!(),
                    }),
                _ => unreachable!(),
            }),
            *var_type
        );
    }

    #[test]
    fn infer_structural() {
        let inferred = infer_type_from_script_ignore_errors("42");
        assert_eq!(
            inferred,
            Type::from(LiteralTypeDefinition::Integer(42.into()),)
        );

        let inferred = infer_type_from_script_ignore_errors("@endpoint");
        assert_eq!(
            inferred,
            Type::from(LiteralTypeDefinition::Endpoint(
                Endpoint::from_str("@endpoint").unwrap()
            ),)
        );

        let inferred = infer_type_from_script_ignore_errors(r#""hello world""#);
        assert_eq!(
            inferred,
            Type::from(LiteralTypeDefinition::Text("hello world".into()),)
        );

        let inferred = infer_type_from_script_ignore_errors("true");
        assert_eq!(
            inferred,
            Type::from(LiteralTypeDefinition::Boolean(true.into()),)
        );
    }

    #[test]
    fn statements_expression() {
        let inferred = infer_type_from_script_ignore_errors("10; 20; 30");
        assert_eq!(
            inferred,
            Type::from(LiteralTypeDefinition::Integer(30.into()),)
        );

        let inferred = infer_type_from_script_ignore_errors("10; 20; 30;");
        assert_eq!(inferred, Type::core(CoreLibBaseTypeId::Unit));
    }

    #[test]
    fn var_declaration() {
        let inferred = infer_type_from_script_ignore_errors("var x = 42");
        assert_eq!(
            inferred,
            Type::from(LiteralTypeDefinition::Integer(42.into()),)
        );
    }

    #[test]
    fn shared_containers() {
        let inferred = infer_type_from_script_ignore_errors("shared 42");
        assert_eq!(
            inferred,
            Type::from(TypeDefinitionWithMetadata::new(
                LiteralTypeDefinition::Integer(42.into()).into(),
                TypeMetadata::Shared {
                    mutability: SharedContainerMutability::Immutable,
                    ownership: SharedContainerOwnership::Owned
                },
            ))
        );

        let inferred = infer_type_from_script_ignore_errors("shared mut 42");
        assert_eq!(
            inferred,
            Type::from(TypeDefinitionWithMetadata::new(
                LiteralTypeDefinition::Integer(42.into()).into(),
                TypeMetadata::Shared {
                    mutability: SharedContainerMutability::Mutable,
                    ownership: SharedContainerOwnership::Owned
                },
            ))
        );
    }

    #[test]
    fn shared_container_refs() {
        let inferred = infer_type_from_script_ignore_errors("'shared 42");
        assert_eq!(
            inferred,
            Type::from(TypeDefinitionWithMetadata::new(
                LiteralTypeDefinition::Integer(42.into()).into(),
                TypeMetadata::Shared {
                    mutability: SharedContainerMutability::Immutable,
                    ownership: SharedContainerOwnership::Referenced(
                        ReferenceMutability::Immutable
                    )
                }
            ))
        );

        let inferred = infer_type_from_script_ignore_errors("'shared mut 42");
        assert_eq!(
            inferred,
            Type::from(TypeDefinitionWithMetadata::new(
                LiteralTypeDefinition::Integer(42.into()).into(),
                TypeMetadata::Shared {
                    mutability: SharedContainerMutability::Mutable,
                    ownership: SharedContainerOwnership::Referenced(
                        ReferenceMutability::Immutable
                    )
                },
            ))
        );

        let inferred =
            infer_type_from_script_ignore_errors("'mut shared mut 42");
        assert_eq!(
            inferred,
            Type::from(TypeDefinitionWithMetadata::new(
                LiteralTypeDefinition::Integer(42.into()).into(),
                TypeMetadata::Shared {
                    mutability: SharedContainerMutability::Mutable,
                    ownership: SharedContainerOwnership::Referenced(
                        ReferenceMutability::Mutable
                    )
                },
            ))
        );
    }

    #[test]
    fn invalid_shared_container_refs() {
        // shared ref to local value not allowed
        let inferred = infer_from_script("'42");
        assert_eq!(
            inferred.unwrap_err().errors[0],
            SpannedTypeError::from(TypeError::InvalidSharedReference)
        );

        // mutable shared ref to immutable shared value not allowed
        let inferred = infer_from_script("'mut shared 42");
        assert_eq!(
            inferred.unwrap_err().errors[0],
            SpannedTypeError::from(TypeError::InvalidSharedReference)
        );
    }

    #[test]
    fn unbox() {
        let inferred = infer_from_script("*(shared (shared 42))");
        assert_eq!(
            inferred.to_type(),
            Type::from(TypeDefinitionWithMetadata::new(
                LiteralTypeDefinition::Integer(42.into()).into(),
                TypeMetadata::Shared {
                    mutability: SharedContainerMutability::Immutable,
                    ownership: SharedContainerOwnership::Owned
                },
            ))
        );
    }

    #[test]
    fn invalid_unbox() {
        let inferred = infer_from_script("*42");
        assert_eq!(
            inferred.unwrap_err().errors[0],
            SpannedTypeError::from(TypeError::invalid_unbox_type(Type::from(
                LiteralTypeDefinition::Integer(42.into())
            )))
        );

        let inferred = infer_from_script("*(shared 42)");
        assert_eq!(
            inferred.unwrap_err().errors[0],
            SpannedTypeError::from(TypeError::invalid_unbox_type(Type::from(
                LiteralTypeDefinition::Integer(42.into())
            )))
        );
    }

    #[test]
    fn var_declaration_and_access() {
        let inferred = infer_type_from_script_ignore_errors("var x = 42; x");
        assert_eq!(
            inferred,
            Type::from(LiteralTypeDefinition::Integer(42.into()),)
        );

        let inferred =
            infer_type_from_script_ignore_errors("var y: integer = 100u8; y");
        assert_eq!(inferred, Type::core(CoreLibBaseTypeId::Integer));
    }

    #[test]
    fn var_declaration_with_type_annotation() {
        let inferred =
            infer_type_from_script_ignore_errors("var x: integer = 42");
        assert_eq!(inferred, Type::core(CoreLibBaseTypeId::Integer));
        let inferred =
            infer_type_from_script_ignore_errors("var x: integer/u8 = 42");
        assert_eq!(
            inferred,
            Type::core(CoreLibVariantTypeId::Integer(IntegerTypeVariant::U8))
        );
        let inferred =
            infer_type_from_script_ignore_errors("var x: decimal = 42");
        assert_eq!(inferred, Type::core(CoreLibBaseTypeId::Decimal));

        let inferred =
            infer_type_from_script_ignore_errors("var x: boolean = true");
        assert_eq!(inferred, Type::core(CoreLibBaseTypeId::Boolean));

        let inferred =
            infer_type_from_script_ignore_errors(r#"var x: text = "hello""#);
        assert_eq!(inferred, Type::core(CoreLibBaseTypeId::Text));
    }

    #[test]
    fn property_assignment() {
        let src = r#"
        var a = { b: 42 };
        a.b = 100
        "#;
        let inferred_type = infer_type_from_script_ignore_errors(src); // should be 100 of b property type
        assert_eq!(
            inferred_type,
            Type::from(LiteralTypeDefinition::Integer(Integer::from(100)),)
        );
    }

    #[test]
    fn var_declaration_reassignment() {
        let src = r#"
        var a: text | integer = 42;
        a = "hello";
        a = 45;
        "#;
        let metadata = ast_for_script(src).metadata;
        let metadata = metadata.borrow();
        let var = metadata.variable_metadata(0).unwrap();
        let var_type = var.var_type.as_ref().unwrap();
        assert_eq!(
            var_type,
            &Type::from(TypeDefinition::Union(UnionTypeDefinition(vec![
                Type::core(CoreLibBaseTypeId::Text),
                Type::core(CoreLibBaseTypeId::Integer)
            ])))
        );
    }

    #[test]
    fn assignment_type_mismatch() {
        let src = r#"
        var a: integer = 42;
        a = "hello"; // type error
        "#;
        let errors = errors_for_script(src);
        let _ = errors.first().unwrap();

        // TODO:
        // assert_matches!(
        //     &error.error,
        //     TypeError::AssignmentTypeMismatch {
        //         expected,
        //         found
        //     } if *annotated_type == core_lib_type(CoreLibTypeId::Integer(None))
        //       && assigned_type == &Type::structural(LiteralTypeDefinition::Text("hello".to_string().into()), TypeMetadata::default())
        // );
    }

    #[test]
    fn binary_operation() {
        let inferred = infer_type_from_script_ignore_errors("10 + 32");
        assert_eq!(inferred, Type::core(CoreLibBaseTypeId::Integer));

        let inferred = infer_type_from_script_ignore_errors(r#"10 + "test""#);
        assert_eq!(inferred, Type::core(CoreLibBaseTypeId::Never));
    }

    #[test]
    fn infer_suffix_typed_literal() {
        let inferred_type =
            infer_type_from_script_ignore_errors("entity X = 42u8");
        assert!(
            has_entity_type_definition(
                &inferred_type,
                EntityTypeDefinition::new(
                    LiteralTypeDefinition::TypedInteger(TypedInteger::U8(42))
                        .into(),
                    "X".to_string()
                )
            ),
            "Expected nominal type definition with typed integer literal, got {:?}",
            inferred_type
        );

        let inferred_type =
            infer_type_from_script_ignore_errors("entity X = 42i32");
        assert!(
            has_entity_type_definition(
                &inferred_type,
                EntityTypeDefinition::new(
                    LiteralTypeDefinition::TypedInteger(TypedInteger::I32(42))
                        .into(),
                    "X".to_string()
                )
            ),
            "Expected nominal type definition with typed integer literal, got {:?}",
            inferred_type
        );

        let inferred_type =
            infer_type_from_script_ignore_errors("entity X = 42.69f32");
        assert!(
            has_entity_type_definition(
                &inferred_type,
                EntityTypeDefinition::new(
                    LiteralTypeDefinition::TypedDecimal(TypedDecimal::from(
                        42.69_f32
                    ))
                    .into(),
                    "X".to_string()
                )
            ),
            "Expected nominal type definition with typed decimal literal, got {:?}",
            inferred_type
        );
    }

    fn has_entity_type_definition(
        ty: &Type,
        expected_definition: EntityTypeDefinition,
    ) -> bool {
        if let Type::Entity(container) = ty {
            container.entity_definition().deref() == &expected_definition
        } else {
            false
        }
    }

    #[test]
    fn infer_type_simple_literal() {
        let inferred_type =
            infer_type_from_script_ignore_errors("entity X = 42");

        assert!(
            has_entity_type_definition(
                &inferred_type,
                EntityTypeDefinition::new(
                    LiteralTypeDefinition::Integer(Integer::from(42)).into(),
                    "X".to_string()
                )
            ),
            "Expected nominal type definition with integer literal, got {:?}",
            inferred_type
        );

        let inferred_type =
            infer_type_from_script_ignore_errors("entity X = 3/4");
        assert!(
            has_entity_type_definition(
                &inferred_type,
                EntityTypeDefinition::new(
                    LiteralTypeDefinition::Decimal(
                        Decimal::try_from_string("3/4").unwrap()
                    )
                    .into(),
                    "X".to_string()
                )
            ),
            "Expected nominal type definition with decimal literal, got {:?}",
            inferred_type
        );

        let inferred_type =
            infer_type_from_script_ignore_errors("entity X = true");
        assert!(
            has_entity_type_definition(
                &inferred_type,
                EntityTypeDefinition::new(
                    LiteralTypeDefinition::Boolean(true.into()).into(),
                    "X".to_string()
                )
            ),
            "Expected nominal type definition with boolean literal, got {:?}",
            inferred_type
        );

        let inferred_type =
            infer_type_from_script_ignore_errors("entity X = false");
        assert!(
            has_entity_type_definition(
                &inferred_type,
                EntityTypeDefinition::new(
                    LiteralTypeDefinition::Boolean(false.into()).into(),
                    "X".to_string(),
                ),
            ),
            "Expected nominal type definition with boolean literal, got {:?}",
            inferred_type,
        );

        let inferred_type =
            infer_type_from_script_ignore_errors(r#"entity X = "hello""#);
        assert!(
            has_entity_type_definition(
                &inferred_type,
                EntityTypeDefinition::new(
                    LiteralTypeDefinition::Text("hello".to_string().into())
                        .into(),
                    "X".to_string()
                )
            ),
            "Expected nominal type definition with text literal, got {:?}",
            inferred_type
        );
    }

    #[test]
    // TODO #451 resolve intersection and union types properly
    // by merging the member types if one is base (one level higher) than the other
    fn infer_intersection_type_expression() {
        let inferred_type =
            infer_type_from_script_ignore_errors("entity X = integer/u8 & 42");
        assert!(
            has_entity_type_definition(
                &inferred_type,
                EntityTypeDefinition::new(
                    TypeDefinition::Intersection(IntersectionTypeDefinition(
                        vec![
                            Type::core(CoreLibVariantTypeId::Integer(
                                IntegerTypeVariant::U8
                            )),
                            Type::from(LiteralTypeDefinition::Integer(
                                Integer::from(42)
                            ),)
                        ]
                    ))
                    .into(),
                    "X".to_string()
                )
            ),
            "Expected nominal type definition with intersection of integer/u8 and integer literal, got {:?}",
            inferred_type
        );
    }

    #[test]
    fn infer_union_type_expression() {
        let inferred_type = infer_type_from_script_ignore_errors(
            "entity X = integer/u8 | decimal",
        );
        assert!(has_entity_type_definition(
            &inferred_type,
            EntityTypeDefinition::new(
                TypeDefinition::Union(UnionTypeDefinition(vec![
                    Type::core(CoreLibVariantTypeId::Integer(
                        IntegerTypeVariant::U8
                    )),
                    Type::core(CoreLibBaseTypeId::Decimal)
                ]))
                .into(),
                "X".to_string()
            )
        ));
    }

    #[test]
    fn infer_empty_struct_type_expression() {
        let inferred_type =
            infer_type_from_script_ignore_errors("entity X = {}");
        assert!(has_entity_type_definition(
            &inferred_type,
            EntityTypeDefinition::new(
                TypeDefinition::Map(vec![].into_iter().collect()).into(),
                "X".to_string(),
            ),
        ));
    }

    #[test]
    fn infer_struct_type_expression() {
        let inferred_type = infer_type_from_script_ignore_errors(
            "entity X = { a: integer/u8, b: decimal }",
        );
        assert!(
            has_entity_type_definition(
                &inferred_type,
                EntityTypeDefinition::new(
                    TypeDefinition::Map(
                        vec![
                            (
                                Type::from(LiteralTypeDefinition::Text(
                                    "a".into()
                                ),),
                                Type::core(CoreLibVariantTypeId::Integer(
                                    IntegerTypeVariant::U8
                                )),
                            ),
                            (
                                Type::from(LiteralTypeDefinition::Text(
                                    "b".into()
                                ),),
                                Type::core(CoreLibBaseTypeId::Decimal)
                            )
                        ]
                        .into_iter()
                        .collect()
                    )
                    .into(),
                    "X".to_string()
                )
            )
        );
    }

    #[test]
    fn infer_variable_declaration() {
        /*
        const x = 10
        */
        let mut expr =
            DatexExpressionData::VariableDeclaration(VariableDeclaration {
                id: None,
                kind: VariableKind::Const,
                name: "x".to_string(),
                type_annotation: None,
                init_expression: (DatexExpressionData::Integer(Integer::from(
                    10,
                ))
                .with_default_span()),
            })
            .with_default_span();

        let infer = ast_for_expression(&mut expr);

        // check that the variable metadata has been updated
        let metadata = infer.metadata.borrow();
        let var_metadata = metadata.variable_metadata(0).unwrap();
        assert_eq!(
            var_metadata.var_type,
            Some(Type::from(LiteralTypeDefinition::Integer(Integer::from(
                10
            )),)),
        );
    }

    #[test]
    fn infer_binary_expression_types() {
        let integer = Type::core(CoreLibBaseTypeId::Integer);
        let decimal = Type::core(CoreLibBaseTypeId::Decimal);

        // integer - integer = integer
        let mut expr = DatexExpressionData::BinaryOperation(BinaryOperation {
            operator: BinaryOperator::Arithmetic(ArithmeticOperator::Subtract),
            left: (DatexExpressionData::Integer(Integer::from(1))
                .with_default_span()),
            right: (DatexExpressionData::Integer(Integer::from(2))
                .with_default_span()),
            ty: None,
        })
        .with_default_span();

        assert_eq!(infer_from_expression(&mut expr), integer);

        // decimal + decimal = decimal
        let mut expr = DatexExpressionData::BinaryOperation(BinaryOperation {
            operator: BinaryOperator::Arithmetic(ArithmeticOperator::Add),
            left: (DatexExpressionData::Decimal(Decimal::from(1.0))
                .with_default_span()),
            right: (DatexExpressionData::Decimal(Decimal::from(2.0))
                .with_default_span()),
            ty: None,
        })
        .with_default_span();
        assert_eq!(infer_from_expression(&mut expr), decimal);

        // integer + decimal = type error
        let mut expr = DatexExpressionData::BinaryOperation(BinaryOperation {
            operator: BinaryOperator::Arithmetic(ArithmeticOperator::Add),
            left: (DatexExpressionData::Integer(Integer::from(1))
                .with_default_span()),
            right: (DatexExpressionData::Decimal(Decimal::from(2.0))
                .with_default_span()),
            ty: None,
        })
        .with_default_span();

        assert!(matches!(
            *errors_for_expression(&mut expr).first().unwrap().error,
            TypeError::MismatchedOperands(_)
        ));
    }

    #[test]
    fn addition_to_immutable_ref() {
        let script = "const a = &42; *a += 1;";
        let result = errors_for_script(script);
        assert_matches!(
            *result.first().unwrap().error,
            TypeError::AssignmentToImmutableReference { .. }
        );
    }

    #[test]
    #[ignore = "Implement property access type inference first"]
    fn mutation_of_immutable_value() {
        let script = "const a = {x: 10}; a.x = 20;";
        let result = errors_for_script(script);
        assert_matches!(
            *result.first().unwrap().error,
            TypeError::AssignmentToImmutableValue { .. }
        );
    }

    #[test]
    #[ignore = "Implement property access type inference first"]
    fn mutation_of_mutable_value() {
        let script = "const a = mut {x: 10}; a.x = 20;";
        let result = errors_for_script(script);
        assert_matches!(
            *result.first().unwrap().error,
            TypeError::AssignmentToImmutableValue { .. }
        );
    }
}
