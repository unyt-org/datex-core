use crate::ast::binary_operation::BinaryOperator;
use crate::ast::{DatexExpression, TypeExpression};
use crate::compiler::precompiler::AstMetadata;
use crate::libs::core::{
    CoreLibPointerId, get_core_lib_type, get_core_lib_type_reference,
};
use crate::values::core_values::r#type::Type;
use crate::values::core_values::r#type::structural_type_definition::StructuralTypeDefinition;
use crate::values::pointer::PointerAddress;
use crate::values::type_container::TypeContainer;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Debug)]
pub enum TypeError {
    MismatchedOperands(TypeContainer, TypeContainer),
}

struct ResolvedPointer {}

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
        DatexExpression::TypeExpression(type_expr) => {
            infer_type_expression_type(type_expr, metadata)?
        }
        DatexExpression::TypeDeclaration {
            id,
            name: _,
            value,
            hoisted: _,
        } => {
            let type_def = infer_type_expression_type(value, metadata.clone())?;
            let type_id = id.expect("TypeDeclaration should have an id assigned during precompilation");
            metadata
                .borrow_mut()
                .variable_metadata_mut(type_id)
                .expect("TypeDeclaration should have variable metadata")
                .var_type = Some(type_def.clone());
            type_def
        }
        DatexExpression::Variable(id, _) => {
            let var_id = *id;
            let metadata = metadata.borrow();
            metadata
                .variable_metadata(var_id)
                .expect("Variable should have variable metadata")
                .var_type
                .clone()
                .expect("Variable type should have been inferred already")
        }
        DatexExpression::VariableDeclaration {
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
                let annotated_type = infer_type_expression_type(
                    type_annotation,
                    metadata.clone(),
                )?;
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
        e => panic!("Type inference not implemented for expression: {:?}", e),
        // _ => get_core_lib_type(CoreLibPointerId::Unit), // other expressions not handled yet
    })
}

fn infer_type_expression_type(
    ast: &mut TypeExpression,
    metadata: Rc<RefCell<AstMetadata>>,
) -> Result<TypeContainer, TypeError> {
    Ok(match ast {
        TypeExpression::Variable(id, _) => {
            let var_id = *id;
            let metadata = metadata.borrow();
            metadata
                .variable_metadata(var_id)
                .expect("Type variable should have variable metadata")
                .var_type
                .clone()
                .expect("Type variable type should have been inferred already")
        }
        TypeExpression::GetReference(pointer_address) => {
            if matches!(pointer_address, PointerAddress::Internal(_)) {
                get_core_lib_type(CoreLibPointerId::from(
                    &pointer_address.to_owned(),
                ))
            } else {
                panic!("GetReference not supported yet")
            }
        }
        _ => panic!(
            "Type inference not implemented for type expression: {:?}",
            ast
        ),
    })
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
                Err(TypeError::MismatchedOperands(lhs_type, rhs_type))
            }
        }

        _ => todo!(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::binary_operation::ArithmeticOperator;
    use crate::ast::error::src;
    use crate::ast::{VariableKind, parse};
    use crate::compiler::precompiler::{
        AstWithMetadata, PrecompilerScopeStack, precompile_ast,
    };
    use crate::libs::core::{CoreLibPointerId, get_core_lib_type};
    use crate::values::core_value::CoreValue;
    use crate::values::core_values::integer::integer::Integer;
    use crate::values::core_values::integer::typed_integer::IntegerTypeVariant;
    use datex_core::values::core_values::boolean::Boolean;
    use datex_core::values::core_values::decimal::decimal::Decimal;

    fn infer_get_type(expr: &mut DatexExpression) -> Type {
        infer_expression_type(
            expr,
            Rc::new(RefCell::new(AstMetadata::default())),
        )
        .map(|tc| tc.as_type())
        .expect("TypeContainer should contain a Type")
    }

    /// Parses the given source code into an AST with metadata.
    fn parse_and_precompile(src: &str) -> AstWithMetadata {
        let ast = parse(src).expect("Invalid expression");
        precompile_ast(
            ast,
            Rc::new(RefCell::new(AstMetadata::default())),
            &mut PrecompilerScopeStack::default(),
        )
        .unwrap()
    }

    /// Helpers to infer the type of a type expression from source code.
    /// The source code should be a type expression, e.g. "integer/u8".
    /// The function wraps the type expression in a type declaration to parse it.
    fn infer_type_expr_from_str(src: &str) -> TypeContainer {
        let src = format!("type X = {}", src);
        let ast_with_metadata = parse_and_precompile(&src);
        let mut expr = ast_with_metadata.ast;
        infer_type_expression_type(
            match &mut expr {
                DatexExpression::TypeDeclaration { value, .. } => value,
                _ => unreachable!(),
            },
            ast_with_metadata.metadata,
        )
        .expect("Type inference failed")
    }

    #[test]
    fn infer_core_type_expression() {
        let inferred_type = infer_type_expr_from_str("integer/u8");
        assert_eq!(
            inferred_type,
            get_core_lib_type(CoreLibPointerId::Integer(Some(
                IntegerTypeVariant::U8,
            )))
        );

        let inferred_type = infer_type_expr_from_str("decimal");
        assert_eq!(
            inferred_type,
            get_core_lib_type(CoreLibPointerId::Decimal(None))
        );

        let inferred_type = infer_type_expr_from_str("boolean");
        assert_eq!(inferred_type, get_core_lib_type(CoreLibPointerId::Boolean));
    }

    /// Tests literal type resolution, as implemented by ValueContainer::try_from
    #[test]
    fn infer_literal_types() {
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
            infer_get_type(&mut DatexExpression::Decimal(Decimal::from(1.23)),),
            Type::structural(StructuralTypeDefinition::Decimal(Decimal::from(
                1.23
            )))
        );

        assert_eq!(
            infer_get_type(&mut DatexExpression::Integer(Integer::from(42)),),
            Type::structural(StructuralTypeDefinition::Integer(Integer::from(
                42
            )))
        );

        assert_eq!(
            infer_get_type(&mut DatexExpression::Array(vec![
                DatexExpression::Integer(Integer::from(1)),
                DatexExpression::Integer(Integer::from(2)),
                DatexExpression::Integer(Integer::from(3))
            ]),),
            Type::structural(StructuralTypeDefinition::Array(vec![
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
            infer_get_type(&mut DatexExpression::Struct(vec![(
                "a".to_string(),
                DatexExpression::Integer(Integer::from(1))
            )]),),
            Type::structural(StructuralTypeDefinition::Struct(vec![(
                "a".to_string(),
                TypeContainer::Type(Type::from(CoreValue::from(
                    Integer::from(1)
                )))
            )]))
        );
    }

    #[test]
    fn infer_binary_expression_types() {
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
            infer_expression_type(
                &mut expr,
                Rc::new(RefCell::new(AstMetadata::default()))
            )
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
            infer_expression_type(
                &mut expr,
                Rc::new(RefCell::new(AstMetadata::default()))
            )
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
            infer_expression_type(
                &mut expr,
                Rc::new(RefCell::new(AstMetadata::default()))
            ),
            Err(TypeError::MismatchedOperands(_, _))
        ));
    }

    #[test]
    fn infer_variable_declaration() {
        /*
        const x = 10
        */
        let expr = DatexExpression::VariableDeclaration {
            id: None,
            kind: VariableKind::Const,
            name: "x".to_string(),
            type_annotation: None,
            init_expression: Box::new(DatexExpression::Integer(Integer::from(
                10,
            ))),
        };

        let ast_with_metadata = precompile_ast(
            expr,
            Rc::new(RefCell::new(AstMetadata::default())),
            &mut PrecompilerScopeStack::default(),
        )
        .unwrap();
        let metadata = ast_with_metadata.metadata;
        let mut expr = ast_with_metadata.ast;

        // check that the expression type is inferred correctly
        assert_eq!(
            infer_expression_type(&mut expr, metadata.clone()).unwrap(),
            Type::structural(StructuralTypeDefinition::Integer(Integer::from(
                10
            )))
            .as_type_container()
        );

        // check that the variable metadata has been updated
        let metadata = metadata.borrow();
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
    fn infer_expression_with_variable() {
        /*
        var x = 10;
        x;
         */
    }
}
