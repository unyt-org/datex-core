use std::{cell::RefCell, ops::Range, rc::Rc};

use crate::{
    ast::structs::{
        expression::{DatexExpression, Map, VariableDeclaration},
        r#type::TypeExpression,
    },
    precompiler::precompiled_ast::{AstMetadata, RichAst},
    type_inferer::{
        error::{
            DetailedTypeErrors, SimpleOrDetailedTypeError, SpannedTypeError,
        },
        options::InferExpressionTypeOptions,
    },
    types::{
        definition, structural_type_definition::StructuralTypeDefinition,
        type_container::TypeContainer,
    },
    values::core_values::{
        boolean::Boolean,
        decimal::{Decimal, typed_decimal::TypedDecimal},
        endpoint::Endpoint,
        integer::{Integer, typed_integer::TypedInteger},
        text::Text,
        r#type::Type,
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
    TypeInferer::new(rich_ast.metadata.clone())
        .infer(&mut rich_ast.ast, options)
        .map(|e| TypeContainer::never())
}

pub struct TypeInferer {
    metadata: Rc<RefCell<AstMetadata>>,
}

impl TypeInferer {
    pub fn new(metadata: Rc<RefCell<AstMetadata>>) -> Self {
        TypeInferer { metadata }
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
}

fn mark_structural_type<E>(
    definition: StructuralTypeDefinition,
) -> Result<VisitAction<E>, SpannedTypeError> {
    Ok(VisitAction::SetTypeAnnotation(
        Type::structural(definition).as_type_container(),
    ))
}
impl TypeExpressionVisitor<SpannedTypeError> for TypeInferer {
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
}
impl ExpressionVisitor<SpannedTypeError> for TypeInferer {
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

        if let Some(specific) = &mut variable_declaration.type_annotation {
            // FIXME check if matches
            Ok(VisitAction::SetTypeAnnotation(
                self.infer_type_expression(specific)?,
            ))
        } else {
            Ok(VisitAction::SetTypeAnnotation(inner))
        }
    }
    // fn visit_map(
    //     &mut self,
    //     map: &mut Map,
    //     span: &Range<usize>,
    // ) -> ExpressionVisitResult<SpannedTypeError> {
    // }
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, rc::Rc};

    use crate::{
        ast::parse,
        precompiler::{
            Precompiler, precompile_ast_simple_error,
            precompiled_ast::{AstMetadata, RichAst},
            scope_stack::PrecompilerScopeStack,
        },
        type_inferer::infer_expression_type_simple_error,
        types::{
            structural_type_definition::StructuralTypeDefinition,
            type_container::TypeContainer,
        },
        values::core_values::r#type::Type,
    };

    fn infer(src: &str) -> RichAst {
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
    fn infer_get_first_type(src: &str) -> TypeContainer {
        let rich_ast = infer(src);
        rich_ast.ast.r#type.clone().expect("No type inferred")
    }

    #[test]
    fn infer_simple_integer() {
        let inferred = infer_get_first_type("42");
        assert_eq!(
            inferred,
            Type::structural(StructuralTypeDefinition::Integer(42.into()))
                .as_type_container()
        );
    }

    #[test]
    fn var_declaration() {
        let inferred = infer_get_first_type("var x = 42");
        assert_eq!(
            inferred,
            Type::structural(StructuralTypeDefinition::Integer(42.into()))
                .as_type_container()
        );
    }
}
