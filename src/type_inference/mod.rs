use std::{cell::RefCell, ops::Range, rc::Rc};

use crate::{
    ast::structs::{
        expression::{
            BinaryOperation, DatexExpression, Statements, TypeDeclaration,
            VariableAccess, VariableDeclaration,
        },
        r#type::TypeExpression,
    },
    libs::core::{CoreLibPointerId, get_core_lib_type},
    precompiler::precompiled_ast::{AstMetadata, RichAst},
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
    metadata: Rc<RefCell<AstMetadata>>,
}

impl TypeInference {
    pub fn new(metadata: Rc<RefCell<AstMetadata>>) -> Self {
        TypeInference { metadata }
    }

    pub fn infer(
        &mut self,
        ast: &mut DatexExpression,
        options: InferExpressionTypeOptions,
    ) -> Result<TypeContainer, SimpleOrDetailedTypeError> {
        let collected_errors = &mut if options.detailed_errors {
            Some(DetailedTypeErrors { errors: vec![] })
        } else {
            None
        };

        let result = self.infer_expression(ast);
        if let Some(collected_errors) = collected_errors.take()
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
}

impl ExpressionVisitor<SpannedTypeError> for TypeInference {
    fn visit_statements(
        &mut self,
        statements: &mut Statements,
        span: &Range<usize>,
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
        span: &Range<usize>,
    ) -> ExpressionVisitResult<SpannedTypeError> {
        mark_type(
            self.variable_type(var_access.id)
                .unwrap_or(TypeContainer::never()),
        )
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
        span: &Range<usize>,
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
        span: &Range<usize>,
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
mod tests {
    use std::{cell::RefCell, rc::Rc, str::FromStr};

    use crate::{
        ast::parse,
        libs::core::{CoreLibPointerId, get_core_lib_type_reference},
        precompiler::{
            precompile_ast_simple_error,
            precompiled_ast::{AstMetadata, RichAst},
            scope_stack::PrecompilerScopeStack,
        },
        references::type_reference::{NominalTypeDeclaration, TypeReference},
        type_inference::infer_expression_type_simple_error,
        types::{
            structural_type_definition::StructuralTypeDefinition,
            type_container::TypeContainer,
        },
        values::core_values::{
            endpoint::Endpoint, integer::typed_integer::IntegerTypeVariant,
            r#type::Type,
        },
    };

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
    fn binary_operation() {
        let inferred = infer_get_type("10 + 32");
        assert_eq!(inferred, TypeContainer::integer());

        let inferred = infer_get_type("10 + 'test'");
        assert_eq!(inferred, TypeContainer::never());
    }
}
