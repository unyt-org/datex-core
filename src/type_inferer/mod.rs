use std::{cell::RefCell, ops::Range, rc::Rc};

use crate::{
    ast::structs::expression::{DatexExpression, Map},
    precompiler::precompiled_ast::AstMetadata,
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
    ast: &mut DatexExpression,
    metadata: Rc<RefCell<AstMetadata>>,
) -> Result<TypeContainer, SpannedTypeError> {
    infer_expression_type(
        ast,
        metadata,
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
    ast: &mut DatexExpression,
    metadata: Rc<RefCell<AstMetadata>>,
) -> Result<TypeContainer, DetailedTypeErrors> {
    infer_expression_type(
        ast,
        metadata,
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
    ast: &mut DatexExpression,
    metadata: Rc<RefCell<AstMetadata>>,
    options: InferExpressionTypeOptions,
) -> Result<TypeContainer, SimpleOrDetailedTypeError> {
    TypeInferer::new(metadata)
        .infer(ast, options)
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
    ) -> Result<(), SimpleOrDetailedTypeError> {
        let collected_errors = &mut if options.detailed_errors {
            Some(DetailedTypeErrors { errors: vec![] })
        } else {
            None
        };

        let result = self.visit_datex_expression(ast);
        if let Some(collected_errors) = collected_errors.take()
            && collected_errors.has_errors()
        {
            Err(SimpleOrDetailedTypeError::Detailed(collected_errors))
        } else {
            result.map_err(SimpleOrDetailedTypeError::from)
        }
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
    // fn visit_map(
    //     &mut self,
    //     map: &mut Map,
    //     span: &Range<usize>,
    // ) -> ExpressionVisitResult<SpannedTypeError> {
    // }
}
