use crate::{
    ast::structs::{
        ResolvedVariable,
        expression::{
            ApplyChain, ComparisonOperation, Conditional, CreateRef,
            DatexExpressionData, Deref, DerefAssignment, FunctionDeclaration,
            List, Map, RemoteExecution, Slot, SlotAssignment, UnaryOperation,
            VariableAssignment, VariantAccess,
        },
        r#type::{
            FixedSizeList, FunctionType, GenericAccess, SliceList,
            TypeVariantAccess,
        },
    },
    global::operators::{
        AssignmentOperator, BinaryOperator, LogicalUnaryOperator,
        UnaryOperator, binary::ArithmeticOperator,
    },
    libs::core::get_core_lib_type_reference,
    references::reference::ReferenceMutability,
    stdlib::rc::Rc,
    type_inference::{error::TypeError, options::ErrorHandling},
    types::definition::TypeDefinition,
};

use core::{cell::RefCell, ops::Range, panic, str::FromStr};

use crate::{
    ast::structs::{
        expression::{
            BinaryOperation, DatexExpression, Statements, TypeDeclaration,
            VariableAccess, VariableDeclaration,
        },
        r#type::{
            Intersection, StructuralList, StructuralMap, TypeExpression, Union,
        },
    },
    compiler::precompiler::precompiled_ast::{AstMetadata, RichAst},
    libs::core::{CoreLibPointerId, get_core_lib_type},
    type_inference::{
        error::{
            DetailedTypeErrors, SimpleOrDetailedTypeError, SpannedTypeError,
        },
        options::InferExpressionTypeOptions,
    },
    types::{
        structural_type_definition::StructuralTypeDefinition,
        type_container::TypeContainer,
    },
    values::{
        core_values::{
            boolean::Boolean,
            decimal::{Decimal, typed_decimal::TypedDecimal},
            endpoint::Endpoint,
            integer::{Integer, typed_integer::TypedInteger},
            text::Text,
            r#type::Type,
        },
        pointer::PointerAddress,
    },
    visitor::{
        VisitAction,
        expression::{ExpressionVisitor, visitable::ExpressionVisitResult},
        type_expression::{
            TypeExpressionVisitor, visitable::TypeExpressionVisitResult,
        },
    },
};

pub mod error;
pub mod options;

// TODO: refactor InferOutcome to a struct containing type, errors and warnings
pub enum InferOutcome {
    Ok(TypeContainer),
    OkWithErrors {
        ty: TypeContainer,
        errors: DetailedTypeErrors,
    },
}
impl From<InferOutcome> for TypeContainer {
    fn from(outcome: InferOutcome) -> Self {
        match outcome {
            InferOutcome::Ok(ty) => ty,
            InferOutcome::OkWithErrors { ty, .. } => ty,
        }
    }
}

pub fn infer_expression_type_simple_error(
    rich_ast: &mut RichAst,
) -> Result<TypeContainer, SpannedTypeError> {
    match infer_expression_type(
        rich_ast,
        InferExpressionTypeOptions {
            detailed_errors: false,
            error_handling: ErrorHandling::FailFast,
        },
    ) {
        Ok(InferOutcome::Ok(ty)) => Ok(ty),
        Ok(InferOutcome::OkWithErrors { ty, .. }) => Ok(ty),
        Err(SimpleOrDetailedTypeError::Simple(e)) => Err(e),
        Err(SimpleOrDetailedTypeError::Detailed(_)) => unreachable!(),
    }
}

pub fn infer_expression_type_detailed_errors(
    rich_ast: &mut RichAst,
) -> Result<TypeContainer, DetailedTypeErrors> {
    match infer_expression_type(
        rich_ast,
        InferExpressionTypeOptions {
            detailed_errors: true,
            error_handling: ErrorHandling::Collect,
        },
    ) {
        Ok(InferOutcome::Ok(ty)) => Ok(ty),
        Ok(InferOutcome::OkWithErrors { .. }) => unreachable!(),
        Err(SimpleOrDetailedTypeError::Detailed(e)) => Err(e),
        Err(SimpleOrDetailedTypeError::Simple(_)) => unreachable!(),
    }
}

pub fn infer_expression_type_with_errors(
    rich_ast: &mut RichAst,
) -> Result<InferOutcome, SimpleOrDetailedTypeError> {
    infer_expression_type(
        rich_ast,
        InferExpressionTypeOptions {
            detailed_errors: true,
            error_handling: ErrorHandling::CollectAndReturnType,
        },
    )
}

/// Infers the type of an expression as precisely as possible.
/// Uses cached type information if available.
fn infer_expression_type(
    rich_ast: &mut RichAst,
    options: InferExpressionTypeOptions,
) -> Result<InferOutcome, SimpleOrDetailedTypeError> {
    TypeInference::new(rich_ast.metadata.clone())
        .infer(&mut rich_ast.ast, options)
}
pub struct TypeInference {
    errors: Option<DetailedTypeErrors>,
    metadata: Rc<RefCell<AstMetadata>>,
}

impl TypeInference {
    pub fn new(metadata: Rc<RefCell<AstMetadata>>) -> Self {
        TypeInference {
            metadata,
            errors: None,
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
                let ty = result.unwrap_or_else(|_| TypeContainer::never());
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
    ) -> Result<TypeContainer, SpannedTypeError> {
        self.visit_datex_expression(expr)?;
        Ok(expr.ty.clone().unwrap_or(TypeContainer::never()))
    }

    fn infer_type_expression(
        &mut self,
        type_expr: &mut TypeExpression,
    ) -> Result<TypeContainer, SpannedTypeError> {
        self.visit_type_expression(type_expr)?;
        Ok(type_expr.ty.clone().unwrap_or(TypeContainer::never()))
    }

    fn variable_type(&self, id: usize) -> Option<TypeContainer> {
        self.metadata
            .borrow()
            .variable_metadata(id)
            .and_then(|meta| meta.var_type.clone())
    }
    fn update_variable_type(&mut self, id: usize, var_type: TypeContainer) {
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
            let action = match error.error {
                TypeError::Unimplemented(_) => {
                    VisitAction::SetTypeRecurseChildNodes(TypeContainer::never())
                }
                _ => VisitAction::SetTypeSkipChildren(TypeContainer::never()),
            };
            collected_errors.errors.push(error);
            Ok(action)
        } else {
            Err(error)
        }
    }
}

fn mark_structural_type<E>(
    definition: StructuralTypeDefinition,
) -> Result<VisitAction<E>, SpannedTypeError> {
    mark_type(Type::structural(definition).as_type_container())
}
fn mark_type<E>(
    type_container: TypeContainer,
) -> Result<VisitAction<E>, SpannedTypeError> {
    Ok(VisitAction::SetTypeSkipChildren(type_container))
}

impl TypeExpressionVisitor<SpannedTypeError> for TypeInference {
    fn visit_integer_type(
        &mut self,
        integer: &mut Integer,
        _: &Range<usize>,
    ) -> TypeExpressionVisitResult<SpannedTypeError> {
        mark_structural_type(StructuralTypeDefinition::Integer(integer.clone()))
    }
    fn visit_typed_integer_type(
        &mut self,
        typed_integer: &mut TypedInteger,
        _: &Range<usize>,
    ) -> TypeExpressionVisitResult<SpannedTypeError> {
        mark_structural_type(StructuralTypeDefinition::TypedInteger(
            typed_integer.clone(),
        ))
    }
    fn visit_decimal_type(
        &mut self,
        decimal: &mut Decimal,
        _: &Range<usize>,
    ) -> TypeExpressionVisitResult<SpannedTypeError> {
        mark_structural_type(StructuralTypeDefinition::Decimal(decimal.clone()))
    }
    fn visit_typed_decimal_type(
        &mut self,
        decimal: &mut TypedDecimal,
        _: &Range<usize>,
    ) -> TypeExpressionVisitResult<SpannedTypeError> {
        mark_structural_type(StructuralTypeDefinition::TypedDecimal(
            decimal.clone(),
        ))
    }
    fn visit_boolean_type(
        &mut self,
        boolean: &mut bool,
        _: &Range<usize>,
    ) -> TypeExpressionVisitResult<SpannedTypeError> {
        mark_structural_type(StructuralTypeDefinition::Boolean(Boolean::from(
            *boolean,
        )))
    }
    fn visit_text_type(
        &mut self,
        text: &mut String,
        _: &Range<usize>,
    ) -> TypeExpressionVisitResult<SpannedTypeError> {
        mark_structural_type(StructuralTypeDefinition::Text(Text::from(
            text.clone(),
        )))
    }
    fn visit_null_type(
        &mut self,
        _: &Range<usize>,
    ) -> TypeExpressionVisitResult<SpannedTypeError> {
        mark_structural_type(StructuralTypeDefinition::Null)
    }
    fn visit_endpoint_type(
        &mut self,
        endpoint: &mut Endpoint,
        _: &Range<usize>,
    ) -> TypeExpressionVisitResult<SpannedTypeError> {
        mark_structural_type(StructuralTypeDefinition::Endpoint(
            endpoint.clone(),
        ))
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
        mark_type(Type::union(members).as_type_container())
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
        mark_type(Type::intersection(members).as_type_container())
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
        mark_structural_type(StructuralTypeDefinition::Map(fields))
    }
    fn visit_structural_list_type(
        &mut self,
        structural_list: &mut StructuralList,
        _: &Range<usize>,
    ) -> TypeExpressionVisitResult<SpannedTypeError> {
        mark_structural_type(StructuralTypeDefinition::List(
            structural_list
                .0
                .iter_mut()
                .map(|elem_type_expr| {
                    self.infer_type_expression(elem_type_expr)
                })
                .collect::<Result<Vec<_>, _>>()?,
        ))
    }

    fn visit_get_reference_type(
        &mut self,
        pointer_address: &mut PointerAddress,
        _: &Range<usize>,
    ) -> TypeExpressionVisitResult<SpannedTypeError> {
        if matches!(pointer_address, PointerAddress::Internal(_)) {
            mark_type(get_core_lib_type(
                CoreLibPointerId::try_from(&pointer_address.to_owned())
                    .unwrap(),
            ))
        } else {
            panic!("GetReference not supported yet")
        }
    }
    fn visit_variable_access_type(
        &mut self,
        var_access: &mut VariableAccess,
        _: &Range<usize>,
    ) -> TypeExpressionVisitResult<SpannedTypeError> {
        mark_type(
            self.variable_type(var_access.id)
                .unwrap_or(TypeContainer::never()),
        )
    }
    fn visit_fixed_size_list_type(
        &mut self,
        fixed_size_list: &mut FixedSizeList,
        span: &Range<usize>,
    ) -> TypeExpressionVisitResult<SpannedTypeError> {
        Err(SpannedTypeError {
            error: TypeError::Unimplemented(
                "FixedSizeList type inference not implemented".into(),
            ),
            span: Some(span.clone()),
        })
    }
    fn visit_function_type(
        &mut self,
        function_type: &mut FunctionType,
        _: &Range<usize>,
    ) -> TypeExpressionVisitResult<SpannedTypeError> {
        let assigned_type =
            self.infer_type_expression(&mut function_type.return_type)?;
        let parameter_types = function_type
            .parameters
            .iter_mut()
            .map(|(key, param_type_expr)| {
                let param_type = self.infer_type_expression(param_type_expr)?;
                Ok((key.clone(), param_type))
            })
            .collect::<Result<Vec<_>, SpannedTypeError>>()?;
        mark_type(
            Type::function(parameter_types, assigned_type).as_type_container(),
        )
    }
    fn visit_generic_access_type(
        &mut self,
        generic_access: &mut GenericAccess,
        span: &Range<usize>,
    ) -> TypeExpressionVisitResult<SpannedTypeError> {
        Err(SpannedTypeError {
            error: TypeError::Unimplemented(
                "GenericAccess type inference not implemented".into(),
            ),
            span: Some(span.clone()),
        })
    }
    fn visit_literal_type(
        &mut self,
        literal: &mut String,
        span: &Range<usize>,
    ) -> TypeExpressionVisitResult<SpannedTypeError> {
        unreachable!(
            "Literal type expressions should have been resolved during precompilation"
        );
    }
    fn visit_ref_mut_type(
        &mut self,
        type_ref_mut: &mut TypeExpression,
        span: &Range<usize>,
    ) -> TypeExpressionVisitResult<SpannedTypeError> {
        let inner_type = self.infer_type_expression(type_ref_mut)?;
        mark_type(inner_type)
    }
    fn visit_ref_type(
        &mut self,
        type_ref: &mut TypeExpression,
        span: &Range<usize>,
    ) -> TypeExpressionVisitResult<SpannedTypeError> {
        let inner_type = self.infer_type_expression(type_ref)?;
        mark_type(inner_type)
    }
    fn visit_slice_list_type(
        &mut self,
        slice_list: &mut SliceList,
        span: &Range<usize>,
    ) -> TypeExpressionVisitResult<SpannedTypeError> {
        Err(SpannedTypeError {
            error: TypeError::Unimplemented(
                "SliceList type inference not implemented".into(),
            ),
            span: Some(span.clone()),
        })
    }
    fn visit_variant_access_type(
        &mut self,
        variant_access: &mut TypeVariantAccess,
        span: &Range<usize>,
    ) -> TypeExpressionVisitResult<SpannedTypeError> {
        Err(SpannedTypeError {
            error: TypeError::Unimplemented(
                "VariantAccess type inference not implemented".into(),
            ),
            span: Some(span.clone()),
        })
    }
}

// FIXME proper implementation of variant access resolution
// currently only works for core lib types, and is hacky.
// We need a good registration system for types and their variants.
fn resolve_type_variant_access(
    base: &PointerAddress,
    variant_name: &str,
) -> Option<PointerAddress> {
    match base {
        PointerAddress::Internal(_) => {
            let base_ref = get_core_lib_type_reference(
                CoreLibPointerId::try_from(base).unwrap(),
            );
            let base_name = base_ref
                .borrow()
                .nominal_type_declaration
                .as_ref()
                .unwrap()
                .name
                .clone();
            CoreLibPointerId::from_str(&format!(
                "{}/{}",
                base_name, variant_name
            ))
            .ok()
            .map(|id| PointerAddress::from(id))
        }
        _ => None,
    }
}

impl ExpressionVisitor<SpannedTypeError> for TypeInference {
    fn visit_create_ref(
        &mut self,
        create_ref: &mut CreateRef,
        _: &Range<usize>,
    ) -> ExpressionVisitResult<SpannedTypeError> {
        let inner_type = self.infer_expression(&mut create_ref.expression)?;
        mark_type(match &inner_type {
            TypeContainer::Type(t) => TypeContainer::Type(Type {
                type_definition: TypeDefinition::Type(Box::new(t.clone())),
                reference_mutability: Some(create_ref.mutability.clone()),
                base_type: None,
            }),
            // TODO #490: check if defined mutability of type reference matches
            TypeContainer::TypeReference(r) => TypeContainer::Type(Type {
                type_definition: TypeDefinition::Reference(r.clone()),
                reference_mutability: Some(create_ref.mutability.clone()),
                base_type: None,
            }),
        })
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
        let mut inferred_type = TypeContainer::unit();

        // Infer type for each statement in order
        for statement in statements.statements.iter_mut() {
            inferred_type = self.infer_expression(statement)?;
        }

        // If the statements block ends with a terminator (semicolon, etc.),
        // it returns the unit type, otherwise, it returns the last inferred type.
        if statements.is_terminated {
            inferred_type = TypeContainer::unit();
        }

        Ok(VisitAction::SetTypeSkipChildren(inferred_type))
    }

    fn visit_variable_access(
        &mut self,
        var_access: &mut VariableAccess,
        _: &Range<usize>,
    ) -> ExpressionVisitResult<SpannedTypeError> {
        mark_type(
            self.variable_type(var_access.id)
                .unwrap_or(TypeContainer::never()),
        )
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
        let assigned_type =
            self.infer_expression(&mut variable_assignment.expression)?;
        let annotated_type =
            self.variable_type(id).unwrap_or(TypeContainer::never());

        match variable_assignment.operator {
            AssignmentOperator::Assign => {
                if !annotated_type.matches_type(&assigned_type) {
                    return Err(SpannedTypeError {
                        error: TypeError::AssignmentTypeMismatch {
                            annotated_type,
                            assigned_type,
                        },
                        span: Some(span.clone()),
                    });
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
        mark_structural_type(StructuralTypeDefinition::Integer(integer.clone()))
    }
    fn visit_typed_integer(
        &mut self,
        typed_integer: &mut TypedInteger,
        _: &Range<usize>,
    ) -> ExpressionVisitResult<SpannedTypeError> {
        mark_structural_type(StructuralTypeDefinition::TypedInteger(
            typed_integer.clone(),
        ))
    }
    fn visit_decimal(
        &mut self,
        decimal: &mut Decimal,
        _: &Range<usize>,
    ) -> ExpressionVisitResult<SpannedTypeError> {
        mark_structural_type(StructuralTypeDefinition::Decimal(decimal.clone()))
    }
    fn visit_typed_decimal(
        &mut self,
        decimal: &mut TypedDecimal,
        _: &Range<usize>,
    ) -> ExpressionVisitResult<SpannedTypeError> {
        mark_structural_type(StructuralTypeDefinition::TypedDecimal(
            decimal.clone(),
        ))
    }
    fn visit_boolean(
        &mut self,
        boolean: &mut bool,
        _: &Range<usize>,
    ) -> ExpressionVisitResult<SpannedTypeError> {
        mark_structural_type(StructuralTypeDefinition::Boolean(Boolean::from(
            *boolean,
        )))
    }
    fn visit_text(
        &mut self,
        text: &mut String,
        _: &Range<usize>,
    ) -> ExpressionVisitResult<SpannedTypeError> {
        mark_structural_type(StructuralTypeDefinition::Text(Text::from(
            text.clone(),
        )))
    }
    fn visit_null(
        &mut self,
        _: &Range<usize>,
    ) -> ExpressionVisitResult<SpannedTypeError> {
        mark_structural_type(StructuralTypeDefinition::Null)
    }
    fn visit_endpoint(
        &mut self,
        endpoint: &mut Endpoint,
        _: &Range<usize>,
    ) -> ExpressionVisitResult<SpannedTypeError> {
        mark_structural_type(StructuralTypeDefinition::Endpoint(
            endpoint.clone(),
        ))
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
                // FIXME check if matches
                let annotated_type = self.infer_type_expression(specific)?;
                if !annotated_type.matches_type(&init_type) {
                    self.record_error(SpannedTypeError::new_with_span(
                        TypeError::AssignmentTypeMismatch {
                            annotated_type: annotated_type.clone(),
                            assigned_type: init_type,
                        },
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
                if left_type.base_type() == right_type.base_type() {
                    mark_type(left_type.base_type())
                } else {
                    Err(SpannedTypeError {
                        error: TypeError::MismatchedOperands(
                            op, left_type, right_type,
                        ),
                        span: Some(span.clone()),
                    })
                }
            }
            _ => {
                //  otherwise, use never type
                self.record_error(SpannedTypeError {
                    error: TypeError::Unimplemented(
                        "Binary operation not implemented".into(),
                    ),
                    span: Some(span.clone()),
                })?;
                mark_type(TypeContainer::never())
            }
        }
    }

    fn visit_type_declaration(
        &mut self,
        type_declaration: &mut TypeDeclaration,
        _: &Range<usize>,
    ) -> ExpressionVisitResult<SpannedTypeError> {
        let type_id = type_declaration.id.expect(
            "TypeDeclaration should have an id assigned during precompilation",
        );
        let var_type = self.variable_type(type_id);
        let type_def = var_type
            .as_ref()
            .expect("TypeDeclaration type should have been inferred already");
        let reference = match &type_def {
            TypeContainer::TypeReference(r) => r,
            _ => {
                panic!("TypeDeclaration var_type should be a TypeReference")
            }
        };

        let inferred_type_def =
            self.infer_type_expression(&mut type_declaration.value)?;

        if type_declaration.kind.is_nominal() {
            match inferred_type_def {
                TypeContainer::Type(t) => {
                    reference.borrow_mut().type_value = t;
                }
                TypeContainer::TypeReference(r) => {
                    reference.borrow_mut().type_value =
                        Type::reference(r, None);
                }
            }
            mark_type(type_def.clone())
        } else {
            self.update_variable_type(type_id, inferred_type_def.clone());
            mark_type(inferred_type_def.clone())
        }
    }

    fn visit_list(
        &mut self,
        list: &mut List,
        _: &Range<usize>,
    ) -> ExpressionVisitResult<SpannedTypeError> {
        mark_structural_type(StructuralTypeDefinition::List(
            list.items
                .iter_mut()
                .map(|elem_type_expr| self.infer_expression(elem_type_expr))
                .collect::<Result<Vec<_>, _>>()?,
        ))
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
        mark_structural_type(StructuralTypeDefinition::Map(fields))
    }
    fn visit_apply_chain(
        &mut self,
        apply_chain: &mut ApplyChain,
        span: &Range<usize>,
    ) -> ExpressionVisitResult<SpannedTypeError> {
        Err(SpannedTypeError {
            error: TypeError::Unimplemented(
                "ApplyChain type inference not implemented".into(),
            ),
            span: Some(span.clone()),
        })
    }
    fn visit_comparison_operation(
        &mut self,
        comparison_operation: &mut ComparisonOperation,
        span: &Range<usize>,
    ) -> ExpressionVisitResult<SpannedTypeError> {
        mark_type(TypeContainer::boolean())
    }
    fn visit_conditional(
        &mut self,
        conditional: &mut Conditional,
        span: &Range<usize>,
    ) -> ExpressionVisitResult<SpannedTypeError> {
        Err(SpannedTypeError {
            error: TypeError::Unimplemented(
                "Conditional type inference not implemented".into(),
            ),
            span: Some(span.clone()),
        })
    }
    fn visit_create_mut(
        &mut self,
        datex_expression: &mut DatexExpression,
        span: &Range<usize>,
    ) -> ExpressionVisitResult<SpannedTypeError> {
        let inner_type = self.infer_expression(datex_expression)?;
        mark_type(match &inner_type {
            TypeContainer::Type(t) => TypeContainer::Type(Type {
                type_definition: TypeDefinition::Type(Box::new(t.clone())),
                reference_mutability: Some(ReferenceMutability::Mutable),
                base_type: None,
            }),
            TypeContainer::TypeReference(r) => TypeContainer::Type(Type {
                type_definition: TypeDefinition::Reference(r.clone()),
                reference_mutability: Some(ReferenceMutability::Mutable),
                base_type: None,
            }),
        })
    }
    fn visit_deref(
        &mut self,
        deref: &mut Deref,
        span: &Range<usize>,
    ) -> ExpressionVisitResult<SpannedTypeError> {
        let inner_type = self.infer_expression(&mut deref.expression)?;
        match &inner_type {
            TypeContainer::Type(t) => {
                if let TypeDefinition::Reference(r) = &t.type_definition {
                    let bor = r.borrow();
                    mark_type(bor.type_value.clone().as_type_container())
                } else {
                    self.record_error(SpannedTypeError {
                        error: TypeError::InvalidDerefType(inner_type),
                        span: Some(span.clone()),
                    })
                }
            }
            TypeContainer::TypeReference(r) => {
                let bor = r.borrow();
                mark_type(bor.type_value.clone().as_type_container())
            }
        }
    }
    fn visit_function_declaration(
        &mut self,
        function_declaration: &mut FunctionDeclaration,
        span: &Range<usize>,
    ) -> ExpressionVisitResult<SpannedTypeError> {
        let annotated_return_type =
            if let Some(return_type) = &mut function_declaration.return_type {
                Some(self.infer_type_expression(return_type)?)
            } else {
                None
            };
        let inferred_return_type = self
            .infer_expression(&mut function_declaration.body)
            .unwrap_or(TypeContainer::never());

        let parameters = function_declaration
            .parameters
            .iter_mut()
            .map(|(name, param_type_expr)| {
                let param_type = self
                    .infer_type_expression(param_type_expr)
                    .unwrap_or(TypeContainer::never());
                (name.clone(), param_type)
            })
            .collect();

        // Check if annotated return type matches inferred return type
        // if an annotated return type is provided
        if let Some(annotated_type) = annotated_return_type {
            // If they match, use the annotated type
            if annotated_type.matches_type(&inferred_return_type) {
                return mark_type(
                    Type::function(parameters, annotated_type)
                        .as_type_container(),
                );
            }
            // If they don't match, record an error
            self.record_error(SpannedTypeError {
                error: TypeError::AssignmentTypeMismatch {
                    annotated_type: annotated_type.clone(),
                    assigned_type: inferred_return_type,
                },
                span: Some(span.clone()),
            })?;
            // Use the annotated type despite the mismatch
            mark_type(
                Type::function(parameters, annotated_type).as_type_container(),
            )
        } else {
            mark_type(
                Type::function(parameters, inferred_return_type)
                    .as_type_container(),
            )
        }
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
                LogicalUnaryOperator::Not => TypeContainer::boolean(),
            },
            UnaryOperator::Arithmetic(_) | UnaryOperator::Bitwise(_) => {
                inner.base_type()
            }
            UnaryOperator::Reference(_) => return Err(SpannedTypeError {
                error: TypeError::Unimplemented(
                    "Unary reference operator type inference not implemented"
                        .into(),
                ),
                span: Some(span.clone()),
            }),
        })
    }
    fn visit_variant_access(
        &mut self,
        variant_access: &mut VariantAccess,
        span: &Range<usize>,
    ) -> ExpressionVisitResult<SpannedTypeError> {
        let pointer_address = match variant_access.base {
            // Handle variant access on a variable
            ResolvedVariable::VariableId(id) => {
                // we expect the variable to be of TypeReference type
                let base_type =
                    self.variable_type(id).ok_or(SpannedTypeError {
                        error: TypeError::Unimplemented(
                            "VariantAccess base variable type not found".into(),
                        ),
                        span: Some(span.clone()),
                    })?;

                // if it's a TypeReference and it has the pointer address set, we can
                // remap the expression to a GetReference
                if let TypeContainer::TypeReference(t) = &base_type
                    && let Some(addr) = &t.borrow().pointer_address
                {
                    Ok(addr.clone())
                } else {
                    // otherwise, unimplemented
                    Err(SpannedTypeError {
                        error: TypeError::Unimplemented(
                            "VariantAccess on Type not implemented".into(),
                        ),
                        span: Some(span.clone()),
                    })
                }
            }
            ResolvedVariable::PointerAddress(ref addr) => Ok(addr.clone()),
        }?;
        let variant_type = resolve_type_variant_access(
            &pointer_address,
            &variant_access.variant,
        )
        .ok_or(SpannedTypeError {
            error: TypeError::SubvariantNotFound(
                variant_access.name.clone(),
                variant_access.variant.clone(),
            ),
            span: Some(span.clone()),
        })?;
        Ok(VisitAction::ReplaceRecurse(DatexExpression::new(
            DatexExpressionData::GetReference(variant_type),
            span.clone(),
        )))
    }

    fn visit_slot(
        &mut self,
        slot: &Slot,
        span: &Range<usize>,
    ) -> ExpressionVisitResult<SpannedTypeError> {
        Err(SpannedTypeError {
            error: TypeError::Unimplemented(
                "Slot type inference not implemented".into(),
            ),
            span: Some(span.clone()),
        })
    }
    fn visit_identifier(
        &mut self,
        identifier: &mut String,
        span: &Range<usize>,
    ) -> ExpressionVisitResult<SpannedTypeError> {
        Ok(VisitAction::SkipChildren)
    }
    fn visit_placeholder(
        &mut self,
        span: &Range<usize>,
    ) -> ExpressionVisitResult<SpannedTypeError> {
        Ok(VisitAction::SkipChildren)
    }
    fn visit_deref_assignment(
        &mut self,
        deref_assignment: &mut DerefAssignment,
        span: &Range<usize>,
    ) -> ExpressionVisitResult<SpannedTypeError> {
        Err(SpannedTypeError {
            error: TypeError::Unimplemented(
                "DerefAssignment type inference not implemented".into(),
            ),
            span: Some(span.clone()),
        })
    }
    fn visit_get_reference(
        &mut self,
        pointer_address: &mut PointerAddress,
        span: &Range<usize>,
    ) -> ExpressionVisitResult<SpannedTypeError> {
        match pointer_address {
            PointerAddress::Internal(_) => mark_type(get_core_lib_type(
                CoreLibPointerId::try_from(&pointer_address.to_owned())
                    .unwrap(),
            )),
            _ => Err(SpannedTypeError {
                error: TypeError::Unimplemented(
                    "GetReference type inference not implemented".into(),
                ),
                span: Some(span.clone()),
            }),
        }
    }
    fn visit_slot_assignment(
        &mut self,
        slot_assignment: &mut SlotAssignment,
        span: &Range<usize>,
    ) -> ExpressionVisitResult<SpannedTypeError> {
        Err(SpannedTypeError {
            error: TypeError::Unimplemented(
                "SlotAssignment type inference not implemented".into(),
            ),
            span: Some(span.clone()),
        })
    }
    fn visit_pointer_address(
        &mut self,
        pointer_address: &PointerAddress,
        span: &Range<usize>,
    ) -> ExpressionVisitResult<SpannedTypeError> {
        Err(SpannedTypeError {
            error: TypeError::Unimplemented(
                "PointerAddress type inference not implemented".into(),
            ),
            span: Some(span.clone()),
        })
    }
    fn visit_remote_execution(
        &mut self,
        remote_execution: &mut RemoteExecution,
        span: &Range<usize>,
    ) -> ExpressionVisitResult<SpannedTypeError> {
        Err(SpannedTypeError {
            error: TypeError::Unimplemented(
                "RemoteExecution type inference not implemented".into(),
            ),
            span: Some(span.clone()),
        })
    }
}

#[cfg(test)]
#[allow(clippy::std_instead_of_core, clippy::std_instead_of_alloc)]
mod tests {
    use std::{
        assert_matches::assert_matches, cell::RefCell, rc::Rc, str::FromStr,
    };

    use crate::{
        ast::{
            parse,
            parse_result::ValidDatexParseResult,
            spanned::Spanned,
            structs::expression::{
                BinaryOperation, DatexExpression, DatexExpressionData, List,
                Map, VariableDeclaration, VariableKind,
            },
        },
        compiler::precompiler::{
            precompile_ast_simple_error,
            precompiled_ast::{AstMetadata, RichAst},
            scope_stack::PrecompilerScopeStack,
        },
        global::operators::{BinaryOperator, binary::ArithmeticOperator},
        libs::core::{
            CoreLibPointerId, get_core_lib_type, get_core_lib_type_reference,
        },
        references::type_reference::{NominalTypeDeclaration, TypeReference},
        type_inference::{
            error::{SpannedTypeError, TypeError},
            infer_expression_type_detailed_errors,
            infer_expression_type_simple_error,
            infer_expression_type_with_errors,
        },
        types::{
            definition::TypeDefinition,
            structural_type_definition::StructuralTypeDefinition,
            type_container::TypeContainer,
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
                r#type::Type,
            },
        },
    };

    /// Infers type errors for the given source code.
    /// Panics if parsing or precompilation succeeds.
    fn errors_for_script(src: &str) -> Vec<SpannedTypeError> {
        let ast = parse(src).unwrap();
        let mut scope_stack = PrecompilerScopeStack::default();
        let ast_metadata = Rc::new(RefCell::new(AstMetadata::default()));
        let mut res =
            precompile_ast_simple_error(ast, &mut scope_stack, ast_metadata)
                .expect("Precompilation failed");
        infer_expression_type_detailed_errors(&mut res)
            .expect_err("Expected type errors")
            .errors
    }

    /// Infers type errors for the given expression.
    /// Panics if precompilation succeeds.
    fn errors_for_expression(
        expr: &mut DatexExpression,
    ) -> Vec<SpannedTypeError> {
        let mut scope_stack = PrecompilerScopeStack::default();
        let ast_metadata = Rc::new(RefCell::new(AstMetadata::default()));
        let mut rich_ast = precompile_ast_simple_error(
            ValidDatexParseResult {
                ast: expr.clone(),
                spans: vec![],
            },
            &mut scope_stack,
            ast_metadata,
        )
        .expect("Precompilation failed");
        infer_expression_type_detailed_errors(&mut rich_ast)
            .expect_err("Expected type errors")
            .errors
    }

    /// Infers the AST of the given source code.
    /// Panics if parsing, precompilation or type inference fails.
    /// Returns the RichAst containing the inferred types.
    fn ast_for_script(src: &str) -> RichAst {
        let ast = parse(src).unwrap();
        let mut scope_stack = PrecompilerScopeStack::default();
        let ast_metadata = Rc::new(RefCell::new(AstMetadata::default()));
        let mut res =
            precompile_ast_simple_error(ast, &mut scope_stack, ast_metadata)
                .expect("Precompilation failed");
        infer_expression_type_simple_error(&mut res)
            .expect("Type inference failed");
        res
    }

    /// Infers the AST of the given expression.
    /// Panics if type inference fails.
    fn ast_for_expression(expr: &mut DatexExpression) -> RichAst {
        let mut scope_stack = PrecompilerScopeStack::default();
        let ast_metadata = Rc::new(RefCell::new(AstMetadata::default()));
        let mut rich_ast = precompile_ast_simple_error(
            ValidDatexParseResult {
                ast: expr.clone(),
                spans: vec![],
            },
            &mut scope_stack,
            ast_metadata,
        )
        .expect("Precompilation failed");
        infer_expression_type_simple_error(&mut rich_ast)
            .expect("Type inference failed");
        rich_ast
    }

    /// Infers the type of the given source code.
    /// Panics if parsing, precompilation. Type errors are collected and ignored.
    /// Returns the inferred type of the full script expression. For example,
    /// for "var x = 42; x", it returns the type of "x", as this is the last expression of the statements.
    /// For "var x = 42;", it returns the never type, as the statement is terminated.
    /// For "10 + 32", it returns the type of the binary operation.
    fn infer_from_script(src: &str) -> TypeContainer {
        let ast = parse(src).unwrap();
        let mut scope_stack = PrecompilerScopeStack::default();
        let ast_metadata = Rc::new(RefCell::new(AstMetadata::default()));
        let mut res =
            precompile_ast_simple_error(ast, &mut scope_stack, ast_metadata)
                .expect("Precompilation failed");
        infer_expression_type_with_errors(&mut res)
            .expect("Type inference failed")
            .into()
    }

    /// Infers the type of the given expression.
    /// Panics if type inference fails.
    fn infer_from_expression(expr: &mut DatexExpression) -> TypeContainer {
        let mut scope_stack = PrecompilerScopeStack::default();
        let ast_metadata = Rc::new(RefCell::new(AstMetadata::default()));

        let mut rich_ast = precompile_ast_simple_error(
            ValidDatexParseResult {
                ast: expr.clone(),
                spans: vec![],
            },
            &mut scope_stack,
            ast_metadata,
        )
        .expect("Precompilation failed");
        infer_expression_type_simple_error(&mut rich_ast)
            .expect("Type inference failed")
    }

    #[test]
    fn variant_access() {
        let src = r#"
        typealias x = integer;
        x/u8
        "#;
        let res = infer_from_script(src);
        assert_eq!(
            res,
            get_core_lib_type(CoreLibPointerId::Integer(Some(
                IntegerTypeVariant::U8
            )))
        );

        let src = r#"
        typealias x = integer;
        x/whatever
        "#;
        let res = errors_for_script(src);
        assert_eq!(
            res.get(0).unwrap().error,
            TypeError::SubvariantNotFound("x".into(), "whatever".into())
        );

        // let src = r#"
        // type x = integer;
        // x/u8
        // "#;
        // let res = errors_for_script(src);
        // println!("Inferred type: {:?}", res);
    }

    #[test]
    fn infer_function_types() {
        let src = r#"
        function add(a: integer, b: integer) -> integer (
            42
        )
        "#;

        let res = infer_from_script(src);
        assert_eq!(
            res.as_type(),
            Type::function(
                vec![
                    (
                        "a".to_string(),
                        get_core_lib_type(CoreLibPointerId::Integer(None))
                    ),
                    (
                        "b".to_string(),
                        get_core_lib_type(CoreLibPointerId::Integer(None))
                    ),
                ],
                get_core_lib_type(CoreLibPointerId::Integer(None))
            )
        );

        let src = r#"
        function add(a: integer, b: integer) (
            42
        )
        "#;

        let res = infer_from_script(src);
        assert_eq!(
            res.as_type(),
            Type::function(
                vec![
                    (
                        "a".to_string(),
                        get_core_lib_type(CoreLibPointerId::Integer(None))
                    ),
                    (
                        "b".to_string(),
                        get_core_lib_type(CoreLibPointerId::Integer(None))
                    ),
                ],
                Type::structural(StructuralTypeDefinition::Integer(
                    Integer::from(42)
                ))
                .as_type_container()
            )
        );
    }

    #[test]
    fn infer_literal_types() {
        assert_eq!(
            infer_from_expression(
                &mut DatexExpressionData::Boolean(true).with_default_span()
            )
            .as_type(),
            Type::structural(StructuralTypeDefinition::Boolean(Boolean(true)))
        );

        assert_eq!(
            infer_from_expression(
                &mut DatexExpressionData::Boolean(false).with_default_span()
            )
            .as_type(),
            Type::structural(StructuralTypeDefinition::Boolean(Boolean(false)))
        );

        assert_eq!(
            infer_from_expression(
                &mut DatexExpressionData::Null.with_default_span()
            )
            .as_type(),
            Type::structural(StructuralTypeDefinition::Null)
        );

        assert_eq!(
            infer_from_expression(
                &mut DatexExpressionData::Decimal(Decimal::from(1.23))
                    .with_default_span()
            )
            .as_type(),
            Type::structural(StructuralTypeDefinition::Decimal(Decimal::from(
                1.23
            )))
        );

        assert_eq!(
            infer_from_expression(
                &mut DatexExpressionData::Integer(Integer::from(42))
                    .with_default_span()
            )
            .as_type(),
            Type::structural(StructuralTypeDefinition::Integer(Integer::from(
                42
            )))
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
            )
            .as_type(),
            Type::structural(StructuralTypeDefinition::List(vec![
                TypeContainer::Type(Type::from(CoreValue::from(
                    Integer::from(1)
                ))),
                TypeContainer::Type(Type::from(CoreValue::from(
                    Integer::from(2)
                ))),
                TypeContainer::Type(Type::from(CoreValue::from(
                    Integer::from(3)
                )))
            ]))
        );

        assert_eq!(
            infer_from_expression(
                &mut DatexExpressionData::Map(Map::new(vec![(
                    DatexExpressionData::Text("a".to_string())
                        .with_default_span(),
                    DatexExpressionData::Integer(Integer::from(1))
                        .with_default_span()
                )]))
                .with_default_span()
            )
            .as_type(),
            Type::structural(StructuralTypeDefinition::Map(vec![(
                Type::structural(StructuralTypeDefinition::Text(
                    "a".to_string().into()
                ))
                .as_type_container(),
                TypeContainer::Type(Type::from(CoreValue::from(
                    Integer::from(1)
                )))
            )]))
        );
    }

    #[test]
    fn nominal_type_declaration() {
        let src = r#"
        type A = integer;
        "#;
        let metadata = ast_for_script(src).metadata;
        let metadata = metadata.borrow();
        let var_a = metadata.variable_metadata(0).unwrap();

        let nominal_ref = TypeReference::nominal(
            Type::reference(
                get_core_lib_type_reference(CoreLibPointerId::Integer(None)),
                None,
            ),
            NominalTypeDeclaration::from("A"),
            None,
        );
        assert_eq!(var_a.var_type, Some(nominal_ref.as_type_container()));
    }

    #[test]
    fn structural_type_declaration() {
        let src = r#"
        typealias A = integer;
        "#;
        let metadata = ast_for_script(src).metadata;
        let metadata = metadata.borrow();
        let var_a = metadata.variable_metadata(0).unwrap();
        let var_type = var_a.var_type.as_ref().unwrap();
        if let TypeContainer::TypeReference(r) = var_type {
            assert_eq!(
                r,
                &get_core_lib_type_reference(CoreLibPointerId::Integer(None))
            );
        } else {
            panic!("Expected TypeReference");
        }

        let inferred_type = infer_from_script("typealias X = integer/u8");
        assert_eq!(
            inferred_type,
            get_core_lib_type(CoreLibPointerId::Integer(Some(
                IntegerTypeVariant::U8,
            )))
        );

        let inferred_type = infer_from_script("typealias X = decimal");
        assert_eq!(
            inferred_type,
            get_core_lib_type(CoreLibPointerId::Decimal(None))
        );

        let inferred_type = infer_from_script("typealias X = boolean");
        assert_eq!(inferred_type, get_core_lib_type(CoreLibPointerId::Boolean));

        let inferred_type = infer_from_script("typealias X = text");
        assert_eq!(inferred_type, get_core_lib_type(CoreLibPointerId::Text));
    }

    #[test]
    fn recursive_types() {
        let src = r#"
        type A = { b: B };
        type B = { a: A };
        "#;
        let metadata = ast_for_script(src).metadata;
        let metadata = metadata.borrow();
        let var = metadata.variable_metadata(0).unwrap();
        let var_type = var.var_type.as_ref().unwrap();
        assert!(matches!(var_type, TypeContainer::TypeReference(_)));
    }

    #[test]
    fn recursive_nominal_type() {
        let src = r#"
        type LinkedList = {
            value: text,
            next: LinkedList | null
        };
        "#;
        let metadata = ast_for_script(src).metadata;
        let metadata = metadata.borrow();
        let var = metadata.variable_metadata(0).unwrap();
        let var_type = var.var_type.as_ref().unwrap();
        assert!(matches!(var_type, TypeContainer::TypeReference(_)));

        // get next field, as wrapped in union
        let next = {
            let var_type_ref = match var_type {
                TypeContainer::TypeReference(r) => r,
                _ => unreachable!(),
            };
            let bor = var_type_ref.borrow();
            let structural_type_definition =
                bor.as_type().structural_type().unwrap();
            let fields = match structural_type_definition {
                StructuralTypeDefinition::Map(fields) => fields,
                _ => unreachable!(),
            };
            let inner_union = match &fields[1].1 {
                TypeContainer::Type(r) => r.clone(),
                _ => unreachable!(),
            }
            .type_definition;
            match inner_union {
                TypeDefinition::Union(members) => {
                    assert_eq!(members.len(), 2);
                    members[0].clone()
                }
                _ => unreachable!(),
            }
        };
        assert_eq!(next, var_type.clone());
    }

    #[test]
    fn infer_structural() {
        let inferred = infer_from_script("42");
        assert_eq!(
            inferred,
            Type::structural(StructuralTypeDefinition::Integer(42.into()))
                .as_type_container()
        );

        let inferred = infer_from_script("@endpoint");
        assert_eq!(
            inferred,
            Type::structural(StructuralTypeDefinition::Endpoint(
                Endpoint::from_str("@endpoint").unwrap()
            ))
            .as_type_container()
        );

        let inferred = infer_from_script("'hello world'");
        assert_eq!(
            inferred,
            Type::structural(StructuralTypeDefinition::Text(
                "hello world".into()
            ))
            .as_type_container()
        );

        let inferred = infer_from_script("true");
        assert_eq!(
            inferred,
            Type::structural(StructuralTypeDefinition::Boolean(true.into()))
                .as_type_container()
        );

        let inferred = infer_from_script("null");
        assert_eq!(
            inferred,
            Type::structural(StructuralTypeDefinition::Null)
                .as_type_container()
        );
    }

    #[test]
    fn statements_expression() {
        let inferred = infer_from_script("10; 20; 30");
        assert_eq!(
            inferred,
            Type::structural(StructuralTypeDefinition::Integer(30.into()))
                .as_type_container()
        );

        let inferred = infer_from_script("10; 20; 30;");
        assert_eq!(inferred, TypeContainer::unit());
    }

    #[test]
    fn var_declaration() {
        let inferred = infer_from_script("var x = 42");
        assert_eq!(
            inferred,
            Type::structural(StructuralTypeDefinition::Integer(42.into()))
                .as_type_container()
        );
    }

    #[test]
    fn var_declaration_and_access() {
        let inferred = infer_from_script("var x = 42; x");
        assert_eq!(
            inferred,
            Type::structural(StructuralTypeDefinition::Integer(42.into()))
                .as_type_container()
        );

        let inferred = infer_from_script("var y: integer = 100u8; y");
        assert_eq!(inferred, TypeContainer::integer());
    }

    #[test]
    fn var_declaration_with_type_annotation() {
        let inferred = infer_from_script("var x: integer = 42");
        assert_eq!(inferred, TypeContainer::integer());
        let inferred = infer_from_script("var x: integer/u8 = 42");
        assert_eq!(
            inferred,
            TypeContainer::typed_integer(IntegerTypeVariant::U8)
        );

        let inferred = infer_from_script("var x: decimal = 42");
        assert_eq!(inferred, TypeContainer::decimal());

        let inferred = infer_from_script("var x: boolean = true");
        assert_eq!(inferred, TypeContainer::boolean());

        let inferred = infer_from_script("var x: text = 'hello'");
        assert_eq!(inferred, TypeContainer::text());
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
            var_type.as_type(),
            Type::union(vec![
                get_core_lib_type(CoreLibPointerId::Text),
                get_core_lib_type(CoreLibPointerId::Integer(None))
            ])
        );
    }

    #[test]
    fn assignment_type_mismatch() {
        let src = r#"
        var a: integer = 42;
        a = "hello"; // type error
        "#;
        let errors = errors_for_script(src);
        let error = errors.first().unwrap();

        assert_matches!(
            &error.error,
            TypeError::AssignmentTypeMismatch {
                annotated_type,
                assigned_type
            } if *annotated_type == get_core_lib_type(CoreLibPointerId::Integer(None))
              && assigned_type.as_type() == Type::structural(StructuralTypeDefinition::Text("hello".to_string().into()))
        );
    }

    #[test]
    fn binary_operation() {
        let inferred = infer_from_script("10 + 32");
        assert_eq!(inferred, TypeContainer::integer());

        let inferred = infer_from_script("10 + 'test'");
        assert_eq!(inferred, TypeContainer::never());
    }

    #[test]
    fn infer_typed_literal() {
        let inferred_type = infer_from_script("type X = 42u8").as_type();
        assert_eq!(
            inferred_type,
            Type::structural(StructuralTypeDefinition::TypedInteger(
                TypedInteger::U8(42)
            ))
        );

        let inferred_type = infer_from_script("type X = 42i32").as_type();
        assert_eq!(
            inferred_type,
            Type::structural(StructuralTypeDefinition::TypedInteger(
                TypedInteger::I32(42)
            ))
        );

        let inferred_type = infer_from_script("type X = 42.69f32").as_type();
        assert_eq!(
            inferred_type,
            Type::structural(StructuralTypeDefinition::TypedDecimal(
                TypedDecimal::from(42.69_f32)
            ))
        );
    }

    #[test]
    fn infer_type_simple_literal() {
        let inferred_type = infer_from_script("type X = 42").as_type();
        assert_eq!(
            inferred_type,
            Type::structural(StructuralTypeDefinition::Integer(Integer::from(
                42
            )))
        );

        let inferred_type = infer_from_script("type X = 3/4").as_type();
        assert_eq!(
            inferred_type,
            Type::structural(StructuralTypeDefinition::Decimal(
                Decimal::from_string("3/4").unwrap()
            ))
        );

        let inferred_type = infer_from_script("type X = true").as_type();
        assert_eq!(
            inferred_type,
            Type::structural(StructuralTypeDefinition::Boolean(Boolean(true)))
        );

        let inferred_type = infer_from_script("type X = false").as_type();
        assert_eq!(
            inferred_type,
            Type::structural(StructuralTypeDefinition::Boolean(Boolean(false)))
        );

        let inferred_type = infer_from_script(r#"type X = "hello""#).as_type();
        assert_eq!(
            inferred_type,
            Type::structural(StructuralTypeDefinition::Text(
                "hello".to_string().into()
            ))
        );
    }

    #[test]
    // TODO #451 resolve intersection and union types properly
    // by merging the member types if one is base (one level higher) than the other
    fn infer_intersection_type_expression() {
        let inferred_type =
            infer_from_script("type X = integer/u8 & 42").as_type();
        assert_eq!(
            inferred_type,
            Type::intersection(vec![
                get_core_lib_type(CoreLibPointerId::Integer(Some(
                    IntegerTypeVariant::U8
                ))),
                Type::structural(StructuralTypeDefinition::Integer(
                    Integer::from(42)
                ))
                .as_type_container()
            ])
        );
    }

    #[test]
    fn infer_union_type_expression() {
        let inferred_type =
            infer_from_script("type X = integer/u8 | decimal").as_type();
        assert_eq!(
            inferred_type,
            Type::union(vec![
                get_core_lib_type(CoreLibPointerId::Integer(Some(
                    IntegerTypeVariant::U8
                ))),
                get_core_lib_type(CoreLibPointerId::Decimal(None))
            ])
        );
    }

    #[test]
    fn infer_empty_struct_type_expression() {
        let inferred_type = infer_from_script("type X = {}").as_type();
        assert_eq!(
            inferred_type,
            Type::structural(StructuralTypeDefinition::Map(vec![]))
        );
    }

    #[test]
    fn infer_struct_type_expression() {
        let inferred_type =
            infer_from_script("type X = { a: integer/u8, b: decimal }")
                .as_type();
        assert_eq!(
            inferred_type,
            Type::structural(StructuralTypeDefinition::Map(vec![
                (
                    Type::structural(StructuralTypeDefinition::Text(
                        "a".to_string().into()
                    ))
                    .as_type_container(),
                    get_core_lib_type(CoreLibPointerId::Integer(Some(
                        IntegerTypeVariant::U8
                    )))
                ),
                (
                    Type::structural(StructuralTypeDefinition::Text(
                        "b".to_string().into()
                    ))
                    .as_type_container(),
                    get_core_lib_type(CoreLibPointerId::Decimal(None))
                )
            ]))
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
                init_expression: Box::new(
                    DatexExpressionData::Integer(Integer::from(10))
                        .with_default_span(),
                ),
            })
            .with_default_span();

        let infer = ast_for_expression(&mut expr);

        // check that the variable metadata has been updated
        let metadata = infer.metadata.borrow();
        let var_metadata = metadata.variable_metadata(0).unwrap();
        assert_eq!(
            var_metadata.var_type,
            Some(
                Type::structural(StructuralTypeDefinition::Integer(
                    Integer::from(10)
                ))
                .as_type_container()
            ),
        );
    }

    #[test]
    fn infer_binary_expression_types() {
        let integer = get_core_lib_type(CoreLibPointerId::Integer(None));
        let decimal = get_core_lib_type(CoreLibPointerId::Decimal(None));

        // integer - integer = integer
        let mut expr = DatexExpressionData::BinaryOperation(BinaryOperation {
            operator: BinaryOperator::Arithmetic(ArithmeticOperator::Subtract),
            left: Box::new(
                DatexExpressionData::Integer(Integer::from(1))
                    .with_default_span(),
            ),
            right: Box::new(
                DatexExpressionData::Integer(Integer::from(2))
                    .with_default_span(),
            ),
            ty: None,
        })
        .with_default_span();

        assert_eq!(infer_from_expression(&mut expr), integer);

        // decimal + decimal = decimal
        let mut expr = DatexExpressionData::BinaryOperation(BinaryOperation {
            operator: BinaryOperator::Arithmetic(ArithmeticOperator::Add),
            left: Box::new(
                DatexExpressionData::Decimal(Decimal::from(1.0))
                    .with_default_span(),
            ),
            right: Box::new(
                DatexExpressionData::Decimal(Decimal::from(2.0))
                    .with_default_span(),
            ),
            ty: None,
        })
        .with_default_span();
        assert_eq!(infer_from_expression(&mut expr), decimal);

        // integer + decimal = type error
        let mut expr = DatexExpressionData::BinaryOperation(BinaryOperation {
            operator: BinaryOperator::Arithmetic(ArithmeticOperator::Add),
            left: Box::new(
                DatexExpressionData::Integer(Integer::from(1))
                    .with_default_span(),
            ),
            right: Box::new(
                DatexExpressionData::Decimal(Decimal::from(2.0))
                    .with_default_span(),
            ),
            ty: None,
        })
        .with_default_span();

        assert!(matches!(
            errors_for_expression(&mut expr).first().unwrap().error,
            TypeError::MismatchedOperands(_, _, _)
        ));
    }
}
