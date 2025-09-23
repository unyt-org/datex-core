use std::cell::RefCell;
use std::rc::Rc;
use crate::ast::{DatexExpression, TypeExpression};
use crate::ast::binary_operation::BinaryOperator;
use crate::compiler::precompiler::{AstMetadata, AstWithMetadata};
use crate::libs::core::{get_core_lib_type, CoreLibPointerId};
use crate::values::core_values::r#type::Type;
use crate::values::core_values::r#type::structural_type_definition::StructuralTypeDefinition;
use crate::values::type_container::TypeContainer;

#[derive(Debug)]
pub enum TypeError {
    MismatchedOperands(TypeContainer, TypeContainer),
}

struct ResolvedPointer {

}


/// Infers the type of an expression as precisely as possible.
/// Uses cached type information if available.
fn infer_expression_type(
    ast: &mut DatexExpression,
    metadata: Rc<RefCell<AstMetadata>>,
) -> Result<TypeContainer, TypeError> {
    Ok(match ast {
        DatexExpression::Null
        | DatexExpression::Boolean(_)
        | DatexExpression::Text(_)
        | DatexExpression::Decimal(_)
        | DatexExpression::Integer(_)
        | DatexExpression::Endpoint(_) => {
            // TODO: this unwrap asserts that try_from succeeds in all cases, but this is not yet guaranteed and tested
            let value = Type::try_from(ast as &_).unwrap();
            TypeContainer::Type(value)
        }
        // composite values
        DatexExpression::Map(map) => {
            todo!()
            // let entries = map
            //     .iter_mut()
            //     .map(|(k, v)| {
            //         // TODO: is unwrap safe here?
            //         let value =
            //             infer_expression_type(v, runtime).unwrap().unwrap();
            //         let key =
            //             infer_expression_type(k, runtime).unwrap().unwrap();
            //         Ok((key, value))
            //     })
            //     // TODO: is unwrap safe here?
            //     .collect::<Result<Vec<(_, _)>, ()>>()
            //     .unwrap();
            // Some(TypeContainer::Type(Type::structural(
            //     StructuralType::Map(entries),
            // )))
        }
        DatexExpression::Struct(structure) => {
            let entries = structure
                .iter_mut()
                .map(|(k, v)| {
                    // TODO: is unwrap safe here?
                    let value =
                        infer_expression_type(v, metadata.clone()).unwrap();
                    Ok((k.clone(), value))
                })
                // TODO: is unwrap safe here?
                .collect::<Result<Vec<(_, _)>, ()>>()
                .unwrap();
            TypeContainer::Type(Type::structural(
                StructuralTypeDefinition::Struct(entries),
            ))
        }
        DatexExpression::Array(arr) => {
            let entries = arr
                .iter_mut()
                .map(|v| {
                    // TODO: is unwrap safe here?
                    infer_expression_type(v, metadata.clone()).unwrap()
                })
                .collect::<Vec<_>>();
            TypeContainer::Type(Type::structural(
                StructuralTypeDefinition::Array(entries),
            ))
        }
        // more complex expressions
        DatexExpression::BinaryOperation(operator, lhs, rhs, cached_type) => {
            infer_binary_expression_type(operator, lhs, rhs, metadata)?
        }
        DatexExpression::VariableDeclaration{
            id,
            kind: _,
            name: _,
            type_annotation,
            init_expression: value,
        } => {
            // infer the type of the value expression
            let init_type = infer_expression_type(value, metadata.clone())?;

            let variable_type = if let Some(type_annotation) = type_annotation {
                // match the inferred type against the annotation
                let annotated_type = infer_type_expression_type(type_annotation, metadata.clone())?;
                todo!("match init_type against annotated_type");
            } else {
                // no annotation, use the inferred type
                init_type
            };

            // store type information for the variable in metadata
            let var_id = id.expect("VariableDeclaration should have an id assigned during precompilation");
            metadata
                .borrow_mut()
                .variable_metadata_mut(var_id)
                .expect("VariableDeclaration should have variable metadata")
                .var_type = Some(variable_type.clone());

            variable_type
        }
        _ => get_core_lib_type(CoreLibPointerId::Unit), // other expressions not handled yet
    })
}

fn infer_type_expression_type(
    ast: &mut TypeExpression,
    metadata: Rc<RefCell<AstMetadata>>,
) -> Result<TypeContainer, TypeError> {
   todo!("Implement type expression inference")
}

fn infer_binary_expression_type(
    operator: &BinaryOperator,
    lhs: &mut Box<DatexExpression>,
    rhs: &mut Box<DatexExpression>,
    metadata: Rc<RefCell<AstMetadata>>,
) -> Result<TypeContainer, TypeError> {
    let lhs_type = infer_expression_type(lhs, metadata.clone())?;
    let rhs_type = infer_expression_type(rhs, metadata)?;

    match operator {
        // numeric-type only operations
        BinaryOperator::Arithmetic(op) => {
            let lhs_base_type = lhs_type.base_type();
            let rhs_base_type = rhs_type.base_type();

            let integer = get_core_lib_type(CoreLibPointerId::Integer(None));
            let decimal = get_core_lib_type(CoreLibPointerId::Decimal(None));

            // // TODO: keep the type as specific as possible here? E.g. 1 + 2 -> 3, not integer
            // lhs and rhs are both integer -> result is integer
            if lhs_base_type == integer && rhs_base_type == integer {
                Ok(integer)
            }
            // lhs and rhs are both decimal -> result is decimal
            else if lhs_base_type == decimal && rhs_base_type == decimal {
                Ok(decimal)
            }
            // otherwise, return type error
            else {
                Err(TypeError::MismatchedOperands(
                    lhs_type,
                    rhs_type,
                ))
            }
        }

        _ => todo!(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::binary_operation::ArithmeticOperator;
    use crate::libs::core::{get_core_lib_type, CoreLibPointerId};
    use crate::values::core_value::CoreValue;
    use crate::values::core_values::integer::integer::Integer;
    use datex_core::values::core_values::boolean::Boolean;
    use datex_core::values::core_values::decimal::decimal::Decimal;
    use crate::ast::{VariableKind};
    use crate::compiler::precompiler::{precompile_ast, PrecompilerScopeStack};

    fn infer_get_type(expr: &mut DatexExpression) -> Type {
        infer_expression_type(expr, Rc::new(RefCell::new(AstMetadata::default())))
            .map(|tc| tc.as_type())
            .expect("TypeContainer should contain a Type")
    }

    /// Tests literal type resolution, as implemented by ValueContainer::try_from
    #[test]
    fn test_infer_literal_types() {
        assert_eq!(
            infer_get_type(&mut DatexExpression::Boolean(true)),
            Type::structural(StructuralTypeDefinition::Boolean(Boolean(true)))
        );

        assert_eq!(
            infer_get_type(&mut DatexExpression::Boolean(false)),
            Type::structural(StructuralTypeDefinition::Boolean(Boolean(false)))
        );

        assert_eq!(
            infer_get_type(&mut DatexExpression::Null),
            Type::structural(StructuralTypeDefinition::Null)
        );

        assert_eq!(
            infer_get_type(
                &mut DatexExpression::Decimal(Decimal::from(1.23)),
            ),
            Type::structural(StructuralTypeDefinition::Decimal(Decimal::from(1.23)))
        );

        assert_eq!(
            infer_get_type(
                &mut DatexExpression::Integer(Integer::from(42)),
            ),
            Type::structural(StructuralTypeDefinition::Integer(Integer::from(42)))
        );

        assert_eq!(
            infer_get_type(
                &mut DatexExpression::Array(vec![
                    DatexExpression::Integer(Integer::from(1)),
                    DatexExpression::Integer(Integer::from(2)),
                    DatexExpression::Integer(Integer::from(3))
                ]),
            ),
            Type::structural(StructuralTypeDefinition::Array(vec![
                TypeContainer::Type(Type::from(CoreValue::from(Integer::from(1)))),
                TypeContainer::Type(Type::from(CoreValue::from(Integer::from(2)))),
                TypeContainer::Type(Type::from(CoreValue::from(Integer::from(3))))
            ]))
        );

        assert_eq!(
            infer_get_type(
                &mut DatexExpression::Struct(vec![(
                    "a".to_string(),
                    DatexExpression::Integer(Integer::from(1))
                )]),
            ),
            Type::structural(StructuralTypeDefinition::Struct(vec![(
                "a".to_string(),
                TypeContainer::Type(Type::from(CoreValue::from(Integer::from(1))))
            )]))
        );
    }

    #[test]
    fn test_infer_binary_expression_types() {
        let integer = get_core_lib_type(CoreLibPointerId::Integer(None));
        let decimal = get_core_lib_type(CoreLibPointerId::Decimal(None));

        // integer - integer = integer
        let mut expr = DatexExpression::BinaryOperation(
            BinaryOperator::Arithmetic(ArithmeticOperator::Subtract),
            Box::new(DatexExpression::Integer(Integer::from(1))),
            Box::new(DatexExpression::Integer(Integer::from(2))),
            None,
        );

        assert_eq!(
            infer_expression_type(&mut expr, Rc::new(RefCell::new(AstMetadata::default())))
                .unwrap(),
            integer
        );

        // decimal + decimal = decimal
        let mut expr = DatexExpression::BinaryOperation(
            BinaryOperator::Arithmetic(ArithmeticOperator::Add),
            Box::new(DatexExpression::Decimal(Decimal::from(1.0))),
            Box::new(DatexExpression::Decimal(Decimal::from(2.0))),
            None,
        );
        assert_eq!(
            infer_expression_type(&mut expr, Rc::new(RefCell::new(AstMetadata::default())))
                .unwrap(),
            decimal
        );

        // integer + decimal = type error
        let mut expr = DatexExpression::BinaryOperation(
            BinaryOperator::Arithmetic(ArithmeticOperator::Add),
            Box::new(DatexExpression::Integer(Integer::from(1))),
            Box::new(DatexExpression::Decimal(Decimal::from(2.0))),
            None,
        );
        assert!(matches!(
            infer_expression_type(&mut expr, Rc::new(RefCell::new(AstMetadata::default()))),
            Err(TypeError::MismatchedOperands(_, _))
        ));
    }

    #[test]
    fn test_infer_variable_declaration() {
        /*
         const x = 10
         */
        let expr = DatexExpression::VariableDeclaration {
            id: None,
            kind: VariableKind::Const,
            name: "x".to_string(),
            type_annotation: None,
            init_expression: Box::new(DatexExpression::Integer(Integer::from(10))),
        };

        let ast_with_metadata = precompile_ast(
            expr,
            Rc::new(RefCell::new(AstMetadata::default())),
            &mut PrecompilerScopeStack::default()
        ).unwrap();
        let metadata = ast_with_metadata.metadata;
        let mut expr = ast_with_metadata.ast;

        // check that the expression type is inferred correctly
        assert_eq!(
            infer_expression_type(&mut expr, metadata.clone())
                .unwrap(),
            Type::structural(StructuralTypeDefinition::Integer(Integer::from(10))).as_type_container()
        );

        // check that the variable metadata has been updated
        let metadata = metadata.borrow();
        let var_metadata = metadata.variable_metadata(0).unwrap();
        assert_eq!(
            var_metadata.var_type,
            Some(Type::structural(StructuralTypeDefinition::Integer(Integer::from(10))).as_type_container()),
        );
    }

    #[test]
    fn test_infer_expression_with_variable() {
        /*
        var x = 10;
        x;
         */
    }
}
