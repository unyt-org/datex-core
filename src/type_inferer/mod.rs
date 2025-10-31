use std::{cell::RefCell, rc::Rc};

use crate::{
    ast::structs::expression::DatexExpression,
    precompiler::precompiled_ast::AstMetadata,
    type_inferer::{
        error::{
            DetailedTypeErrors, SimpleOrDetailedTypeError, SpannedTypeError,
        },
        options::InferExpressionTypeOptions,
    },
    types::type_container::TypeContainer,
    visitor::{
        expression::ExpressionVisitor, type_expression::TypeExpressionVisitor,
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
    TypeInferer::new(metadata).infer(ast, options)
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

        let result = self.visit_datex_expression(ast);
        let result: Result<TypeContainer, SpannedTypeError> =
            Ok(TypeContainer::boolean());
        if let Some(collected_errors) = collected_errors.take()
            && collected_errors.has_errors()
        {
            Err(SimpleOrDetailedTypeError::Detailed(collected_errors))
        } else {
            result.map_err(SimpleOrDetailedTypeError::from)
        }
    }
}

impl TypeExpressionVisitor<SpannedTypeError> for TypeInferer {}
impl ExpressionVisitor<SpannedTypeError> for TypeInferer {}
