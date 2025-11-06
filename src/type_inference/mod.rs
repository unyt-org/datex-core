use crate::{
    ast::structs::expression::VariableAssignment,
    global::operators::AssignmentOperator, stdlib::rc::Rc,
    type_inference::error::TypeError,
};

use core::{cell::RefCell, ops::Range, panic};

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

pub fn infer_expression_type_simple_error(
    rich_ast: &mut RichAst,
) -> Result<TypeContainer, SpannedTypeError> {
    infer_expression_type(
        rich_ast,
        InferExpressionTypeOptions {
            detailed_errors: false,
        },
    )
    .map_err(|error| match error {
        SimpleOrDetailedTypeError::Simple(error) => error,
        _ => unreachable!(), // because detailed_errors: false
    })
}

pub fn infer_expression_type_detailed_errors(
    rich_ast: &mut RichAst,
) -> Result<TypeContainer, DetailedTypeErrors> {
    infer_expression_type(
        rich_ast,
        InferExpressionTypeOptions {
            detailed_errors: true,
        },
    )
    .map_err(|error| match error {
        SimpleOrDetailedTypeError::Detailed(error) => error,
        _ => unreachable!(), // because detailed_errors: true
    })
}

/// Infers the type of an expression as precisely as possible.
/// Uses cached type information if available.
fn infer_expression_type(
    rich_ast: &mut RichAst,
    options: InferExpressionTypeOptions,
) -> Result<TypeContainer, SimpleOrDetailedTypeError> {
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
    ) -> Result<TypeContainer, SimpleOrDetailedTypeError> {
        self.errors = if options.detailed_errors {
            Some(DetailedTypeErrors { errors: vec![] })
        } else {
            None
        };

        let result = self.infer_expression(ast);
        if let Some(collected_errors) = self.errors.take()
            && collected_errors.has_errors()
        {
            Err(SimpleOrDetailedTypeError::Detailed(collected_errors))
        } else {
            result.map_err(SimpleOrDetailedTypeError::from)
        }
    }
    fn infer_expression(
        &mut self,
        expr: &mut DatexExpression,
    ) -> Result<TypeContainer, SpannedTypeError> {
        self.visit_datex_expression(expr)?;
        Ok(expr.r#type.clone().unwrap_or(TypeContainer::never()))
    }
    fn infer_type_expression(
        &mut self,
        type_expr: &mut TypeExpression,
    ) -> Result<TypeContainer, SpannedTypeError> {
        self.visit_type_expression(type_expr)?;
        Ok(type_expr.r#type.clone().unwrap_or(TypeContainer::never()))
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
}

fn mark_structural_type<E>(
    definition: StructuralTypeDefinition,
) -> Result<VisitAction<E>, SpannedTypeError> {
    mark_type(Type::structural(definition).as_type_container())
}
fn mark_type<E>(
    type_container: TypeContainer,
) -> Result<VisitAction<E>, SpannedTypeError> {
    Ok(VisitAction::SetTypeAnnotation(type_container))
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
}

impl ExpressionVisitor<SpannedTypeError> for TypeInference {
    fn handle_expression_error(
        &mut self,
        error: SpannedTypeError,
        _: &DatexExpression,
    ) -> Result<VisitAction<DatexExpression>, SpannedTypeError> {
        if let Some(collected_errors) = &mut self.errors {
            collected_errors.errors.push(error);
            Ok(VisitAction::SetTypeAnnotation(TypeContainer::never()))
        } else {
            Err(error)
        }
    }
    fn visit_statements(
        &mut self,
        statements: &mut Statements,
        _: &Range<usize>,
    ) -> ExpressionVisitResult<SpannedTypeError> {
        let mut inferred_type = TypeContainer::never();
        let size = statements.statements.len();
        for (i, statement) in statements.statements.iter_mut().enumerate() {
            let inner_type = self.infer_expression(statement)?;
            if !statements.is_terminated && i == size - 1 {
                inferred_type = inner_type;
            }
        }
        Ok(VisitAction::SetTypeAnnotation(inferred_type))
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
        _: &Range<usize>,
    ) -> ExpressionVisitResult<SpannedTypeError> {
        let inner =
            self.infer_expression(&mut variable_declaration.init_expression)?;

        let actual_type =
            if let Some(specific) = &mut variable_declaration.type_annotation {
                // FIXME check if matches
                self.infer_type_expression(specific)?
            } else {
                inner
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
        _: &Range<usize>,
    ) -> ExpressionVisitResult<SpannedTypeError> {
        let left_type = self.infer_expression(&mut binary_operation.left)?;
        let right_type = self.infer_expression(&mut binary_operation.right)?;
        // if base types are the same, use that as result type
        if left_type.base_type() == right_type.base_type() {
            mark_type(left_type.base_type())
        } else {
            // otherwise, use never type
            mark_type(TypeContainer::never())
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
        let type_def = self
            .variable_type(type_id)
            .as_ref()
            .expect("TypeDeclaration type should have been inferred already")
            .clone();
        let reference = match &type_def {
            TypeContainer::TypeReference(r) => r.clone(),
            _ => {
                panic!("TypeDeclaration var_type should be a TypeReference")
            }
        };

        let inferred_type_def =
            self.infer_type_expression(&mut type_declaration.value)?;

        println!("Inferring type declaration id {:#?}", reference);
        // let inner_ref = reference.borrow();
        match inferred_type_def {
            TypeContainer::Type(t) => {
                reference.borrow_mut().type_value = t;
            }
            TypeContainer::TypeReference(r) => {
                reference.borrow_mut().type_value = Type::reference(r, None);
                // reference.swap(&r);
            }
        }
        mark_type(type_def)
    }
}

#[cfg(test)]
#[allow(clippy::std_instead_of_core, clippy::std_instead_of_alloc)]
mod tests {
    use std::{
        assert_matches::assert_matches, cell::RefCell, rc::Rc, str::FromStr,
    };

    use crate::{
        ast::parse,
        compiler::precompiler::{
            precompile_ast_simple_error,
            precompiled_ast::{AstMetadata, RichAst},
            scope_stack::PrecompilerScopeStack,
        },
        libs::core::{
            CoreLibPointerId, get_core_lib_type, get_core_lib_type_reference,
        },
        references::type_reference::{NominalTypeDeclaration, TypeReference},
        type_inference::{
            error::{DetailedTypeErrors, SpannedTypeError, TypeError},
            infer_expression_type_detailed_errors,
            infer_expression_type_simple_error,
        },
        types::{
            definition::TypeDefinition,
            structural_type_definition::StructuralTypeDefinition,
            type_container::TypeContainer,
        },
        values::core_values::{
            boolean::Boolean,
            decimal::{Decimal, typed_decimal::TypedDecimal},
            endpoint::Endpoint,
            integer::{
                Integer,
                typed_integer::{IntegerTypeVariant, TypedInteger},
            },
            r#type::Type,
        },
    };

    fn infer_get_errors(src: &str) -> Vec<SpannedTypeError> {
        let ast = parse(src).unwrap();
        let mut scope_stack = PrecompilerScopeStack::default();
        let ast_metadata = Rc::new(RefCell::new(AstMetadata::default()));
        let mut res =
            precompile_ast_simple_error(ast, &mut scope_stack, ast_metadata)
                .expect("Precompilation failed");
        infer_expression_type_detailed_errors(&mut res)
            .err()
            .expect("Expected type errors")
            .errors
    }

    /// Infers the AST of the given source code.
    /// Panics if parsing, precompilation or type inference fails.
    /// Returns the RichAst containing the inferred types.
    fn infer_get_ast(src: &str) -> RichAst {
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

    /// Infers the type of the given source code.
    /// Panics if parsing, precompilation or type inference fails.
    /// Returns the inferred type of the full script expression. For example,
    /// for "var x = 42; x", it returns the type of "x", as this is the last expression of the statements.
    /// For "var x = 42;", it returns the never type, as the statement is terminated.
    /// For "10 + 32", it returns the type of the binary operation.
    fn infer_get_type(src: &str) -> TypeContainer {
        let ast = parse(src).unwrap();
        let mut scope_stack = PrecompilerScopeStack::default();
        let ast_metadata = Rc::new(RefCell::new(AstMetadata::default()));
        let mut res =
            precompile_ast_simple_error(ast, &mut scope_stack, ast_metadata)
                .expect("Precompilation failed");
        infer_expression_type_simple_error(&mut res)
            .expect("Type inference failed")
    }

    #[test]
    fn nominal_type_declaration() {
        let src = r#"
        type A = integer;
        "#;
        let metadata = infer_get_ast(src).metadata;
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
        typedef A = integer;
        "#;
        let metadata = infer_get_ast(src).metadata;
        let metadata = metadata.borrow();
        let var_a = metadata.variable_metadata(0).unwrap();
        let var_type = var_a.var_type.as_ref().unwrap();
        assert!(matches!(var_type, TypeContainer::TypeReference(_)));
        // FIXME assert_eq!(var_type.borrow().pointer_address, Some(CoreLibPointerId::Integer(None).into()));
    }

    #[test]
    fn recursive_types() {
        let src = r#"
        type A = { b: B };
        type B = { a: A };
        "#;
        let metadata = infer_get_ast(src).metadata;
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
        let metadata = infer_get_ast(src).metadata;
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
        let inferred = infer_get_type("42");
        assert_eq!(
            inferred,
            Type::structural(StructuralTypeDefinition::Integer(42.into()))
                .as_type_container()
        );

        let inferred = infer_get_type("@endpoint");
        assert_eq!(
            inferred,
            Type::structural(StructuralTypeDefinition::Endpoint(
                Endpoint::from_str("@endpoint").unwrap()
            ))
            .as_type_container()
        );

        let inferred = infer_get_type("'hello world'");
        assert_eq!(
            inferred,
            Type::structural(StructuralTypeDefinition::Text(
                "hello world".into()
            ))
            .as_type_container()
        );

        let inferred = infer_get_type("true");
        assert_eq!(
            inferred,
            Type::structural(StructuralTypeDefinition::Boolean(true.into()))
                .as_type_container()
        );

        let inferred = infer_get_type("null");
        assert_eq!(
            inferred,
            Type::structural(StructuralTypeDefinition::Null)
                .as_type_container()
        );
    }

    #[test]
    fn statements_expression() {
        let inferred = infer_get_type("10; 20; 30");
        assert_eq!(
            inferred,
            Type::structural(StructuralTypeDefinition::Integer(30.into()))
                .as_type_container()
        );

        let inferred = infer_get_type("10; 20; 30;");
        assert_eq!(inferred, TypeContainer::never());
    }

    #[test]
    fn var_declaration() {
        let inferred = infer_get_type("var x = 42");
        assert_eq!(
            inferred,
            Type::structural(StructuralTypeDefinition::Integer(42.into()))
                .as_type_container()
        );
    }

    #[test]
    fn var_declaration_and_access() {
        let inferred = infer_get_type("var x = 42; x");
        assert_eq!(
            inferred,
            Type::structural(StructuralTypeDefinition::Integer(42.into()))
                .as_type_container()
        );

        let inferred = infer_get_type("var y: integer = 100u8; y");
        assert_eq!(inferred, TypeContainer::integer());
    }

    #[test]
    fn var_declaration_with_type_annotation() {
        let inferred = infer_get_type("var x: integer = 42");
        assert_eq!(inferred, TypeContainer::integer());
        let inferred = infer_get_type("var x: integer/u8 = 42");
        assert_eq!(
            inferred,
            TypeContainer::typed_integer(IntegerTypeVariant::U8)
        );

        let inferred = infer_get_type("var x: decimal = 42");
        assert_eq!(inferred, TypeContainer::decimal());

        let inferred = infer_get_type("var x: boolean = true");
        assert_eq!(inferred, TypeContainer::boolean());

        let inferred = infer_get_type("var x: text = 'hello'");
        assert_eq!(inferred, TypeContainer::text());
    }

    #[test]
    fn var_declaration_reassignment() {
        let src = r#"
        var a: text | integer = 42;
        a = "hello";
        a = 45;
        "#;
        let metadata = infer_get_ast(src).metadata;
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
        let errors = infer_get_errors(src);
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
        let inferred = infer_get_type("10 + 32");
        assert_eq!(inferred, TypeContainer::integer());

        let inferred = infer_get_type("10 + 'test'");
        assert_eq!(inferred, TypeContainer::never());
    }

    #[test]
    fn infer_typed_literal() {
        let inferred_type = infer_get_type("type X = 42u8").as_type();
        assert_eq!(
            inferred_type,
            Type::structural(StructuralTypeDefinition::TypedInteger(
                TypedInteger::U8(42)
            ))
        );

        let inferred_type = infer_get_type("type X = 42i32").as_type();
        assert_eq!(
            inferred_type,
            Type::structural(StructuralTypeDefinition::TypedInteger(
                TypedInteger::I32(42)
            ))
        );

        let inferred_type = infer_get_type("type X = 42.69f32").as_type();
        assert_eq!(
            inferred_type,
            Type::structural(StructuralTypeDefinition::TypedDecimal(
                TypedDecimal::from(42.69_f32)
            ))
        );
    }

    #[test]
    fn infer_type_simple_literal() {
        let inferred_type = infer_get_type("type X = 42").as_type();
        assert_eq!(
            inferred_type,
            Type::structural(StructuralTypeDefinition::Integer(Integer::from(
                42
            )))
        );

        let inferred_type = infer_get_type("type X = 3/4").as_type();
        assert_eq!(
            inferred_type,
            Type::structural(StructuralTypeDefinition::Decimal(
                Decimal::from_string("3/4").unwrap()
            ))
        );

        let inferred_type = infer_get_type("type X = true").as_type();
        assert_eq!(
            inferred_type,
            Type::structural(StructuralTypeDefinition::Boolean(Boolean(true)))
        );

        let inferred_type = infer_get_type("type X = false").as_type();
        assert_eq!(
            inferred_type,
            Type::structural(StructuralTypeDefinition::Boolean(Boolean(false)))
        );

        let inferred_type = infer_get_type(r#"type X = "hello""#).as_type();
        assert_eq!(
            inferred_type,
            Type::structural(StructuralTypeDefinition::Text(
                "hello".to_string().into()
            ))
        );
    }
}
