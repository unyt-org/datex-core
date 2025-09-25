use crate::ast::assignment_operation::AssignmentOperator;
use crate::ast::binary_operation::BinaryOperator;
use crate::ast::{DatexExpression, TypeExpression};
use crate::compiler::precompiler::AstMetadata;
use crate::libs::core::{CoreLibPointerId, get_core_lib_type};
use crate::r#ref::type_reference::TypeReference;
use crate::types::structural_type_definition::StructuralTypeDefinition;
use crate::types::type_container::TypeContainer;
use crate::values::core_values::r#type::Type;
use crate::values::pointer::PointerAddress;
use std::cell::{Ref, RefCell};
use std::rc::Rc;

#[derive(Debug)]
pub enum TypeError {
    MismatchedOperands(TypeContainer, TypeContainer),

    // can not assign value to variable of different type
    AssignmentTypeMismatch {
        annotated_type: TypeContainer,
        assigned_type: TypeContainer,
    },
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
        | DatexExpression::TypedInteger(_)
        | DatexExpression::TypedDecimal(_)
        | DatexExpression::Endpoint(_) => {
            // TODO: this unwrap asserts that try_from succeeds in all cases, but this is not yet guaranteed and tested
            let value = Type::try_from(ast as &_).unwrap();
            TypeContainer::Type(value)
        }
        // composite values
        DatexExpression::Map(map) => {
            todo!("Map type inference not implemented yet");
            // let entries = map
            //     .iter_mut()
            //     .map(|(k, v)| {
            //         let key =
            //             infer_expression_type(k, metadata.clone()).unwrap();
            //         let value =
            //             infer_expression_type(v, metadata.clone()).unwrap();
            //         Ok((key, value))
            //     })
            //     .collect::<Result<Vec<(_, _)>, ()>>()
            //     .unwrap();
            // TypeContainer::Type(Type::structural(
            //     StructuralTypeDefinition::Map(entries),
            // ))
        }
        DatexExpression::Struct(structure) => {
            let entries = structure
                .iter_mut()
                .map(|(k, v)| {
                    let value =
                        infer_expression_type(v, metadata.clone()).unwrap();
                    Ok((k.clone(), value))
                })
                .collect::<Result<Vec<(_, _)>, ()>>()
                .unwrap();
            TypeContainer::Type(Type::structural(
                StructuralTypeDefinition::Struct(entries),
            ))
        }
        DatexExpression::Array(arr) => {
            let entries = arr
                .iter_mut()
                .map(|v| infer_expression_type(v, metadata.clone()).unwrap())
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
            resolve_type_expression_type(type_expr, metadata)?
        }
        DatexExpression::TypeDeclaration {
            id,
            name: _,
            value,
            hoisted: _,
        } => {
            // WIP
            let type_id = id.expect("TypeDeclaration should have an id assigned during precompilation");
            let type_def = {
                let metadata = metadata.borrow();
                let metadata = metadata
                    .variable_metadata(type_id)
                    .expect("TypeDeclaration should have variable metadata");
                metadata.var_type.as_ref().expect(
                    "TypeDeclaration type should have been inferred already",
                ).clone()
            };
            let reference = match &type_def {
                TypeContainer::TypeReference(r) => r.clone(),
                _ => {
                    panic!("TypeDeclaration var_type should be a TypeReference")
                }
            };

            let inferred_type_def =
                resolve_type_expression_type(value, metadata.clone())?;

            match inferred_type_def {
                TypeContainer::Type(t) => {
                    reference.borrow_mut().type_value = t;
                }
                TypeContainer::TypeReference(r) => {
                    reference.swap(&r);
                }
            }

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
                let annotated_type = resolve_type_expression_type(
                    type_annotation,
                    metadata.clone(),
                )?;
                println!(
                    "Matching annotated type {} against inferred type {}",
                    annotated_type, init_type
                );
                if !annotated_type.matches_type(&init_type) {
                    return Err(TypeError::AssignmentTypeMismatch {
                        annotated_type,
                        assigned_type: init_type,
                    });
                }
                annotated_type
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
        DatexExpression::VariableAssignment(operator, id, _, value) => {
            let var_id = id.unwrap();
            let metadata_borrowed = metadata.borrow();
            let var_metadata = metadata_borrowed
                .variable_metadata(var_id)
                .expect("Variable should have variable metadata");
            let var_type = var_metadata
                .var_type
                .as_ref()
                .expect("Variable type should have been inferred already")
                .clone();
            drop(metadata_borrowed);

            let value_type = infer_expression_type(value, metadata.clone())?;

            match operator {
                AssignmentOperator::Assign => {
                    // simple assignment, types must match
                    if !var_type.matches_type(&value_type) {
                        return Err(TypeError::AssignmentTypeMismatch {
                            annotated_type: var_type,
                            assigned_type: value_type,
                        });
                    }
                    value_type
                }
                op => todo!("handle other assignment operators: {:?}", op),
            }
        }
        DatexExpression::Statements(statements) => {
            for stmt in statements.iter_mut() {
                infer_expression_type(&mut stmt.expression, metadata.clone())?;
            }
            get_core_lib_type(CoreLibPointerId::Unit)
        }
        e => panic!("Type inference not implemented for expression: {:?}", e),
        // _ => get_core_lib_type(CoreLibPointerId::Unit), // other expressions not handled yet
    })
}

/// Resolved the type represented by a type expression.
/// This is used in type declarations and type annotations.
/// e.g. `integer/u8`, `{ a: integer, b: decimal }`, `integer | decimal`, etc.
fn resolve_type_expression_type(
    ast: &mut TypeExpression,
    metadata: Rc<RefCell<AstMetadata>>,
) -> Result<TypeContainer, TypeError> {
    // First, try to directly match the type expression to a structural type definition.
    // This covers literals and composite types like structs and arrays.
    // If that fails, handle more complex type expressions like variables, unions, and intersections.
    if let Some(res) = match ast {
        TypeExpression::Integer(value) => {
            Some(StructuralTypeDefinition::Integer(value.clone()))
        }
        TypeExpression::TypedInteger(value) => {
            Some(StructuralTypeDefinition::TypedInteger(value.clone()))
        }
        TypeExpression::Decimal(value) => {
            Some(StructuralTypeDefinition::Decimal(value.clone()))
        }
        TypeExpression::TypedDecimal(value) => {
            Some(StructuralTypeDefinition::TypedDecimal(value.clone()))
        }
        TypeExpression::Boolean(value) => {
            Some(StructuralTypeDefinition::Boolean((*value).into()))
        }
        TypeExpression::Text(value) => Some(value.clone().into()),
        TypeExpression::Null => Some(StructuralTypeDefinition::Null),
        TypeExpression::Endpoint(value) => {
            Some(StructuralTypeDefinition::Endpoint(value.clone()))
        }
        TypeExpression::Struct(fields) => {
            let entries = fields
                .iter_mut()
                .map(|(k, v)| {
                    let value =
                        resolve_type_expression_type(v, metadata.clone())?;
                    Ok((k.clone(), value))
                })
                .collect::<Result<Vec<(_, _)>, TypeError>>()?;
            Some(StructuralTypeDefinition::Struct(entries))
        }
        TypeExpression::Array(members) => {
            let member_types = members
                .iter_mut()
                .map(|m| resolve_type_expression_type(m, metadata.clone()))
                .collect::<Result<Vec<_>, TypeError>>()?;
            Some(StructuralTypeDefinition::Array(member_types))
        }
        TypeExpression::List(entry_type) => {
            let entry_type =
                resolve_type_expression_type(entry_type, metadata.clone())?;
            Some(StructuralTypeDefinition::List(Box::new(entry_type)))
        }
        _ => None,
    } {
        return Ok(Type::structural(res).as_type_container());
    }

    // handle more complex type expressions
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
                get_core_lib_type(
                    CoreLibPointerId::try_from(&pointer_address.to_owned())
                        .unwrap(),
                )
            } else {
                panic!("GetReference not supported yet")
            }
        }
        TypeExpression::Union(members) => {
            let member_types = members
                .iter_mut()
                .map(|m| resolve_type_expression_type(m, metadata.clone()))
                .collect::<Result<Vec<_>, TypeError>>()?;
            Type::union(member_types).as_type_container()
        }
        TypeExpression::Intersection(members) => {
            let member_types = members
                .iter_mut()
                .map(|m| resolve_type_expression_type(m, metadata.clone()))
                .collect::<Result<Vec<_>, TypeError>>()?;
            Type::intersection(member_types).as_type_container()
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
    use std::assert_matches::assert_matches;

    use super::*;
    use crate::ast::binary_operation::ArithmeticOperator;
    use crate::ast::{VariableKind, parse};
    use crate::compiler::error::CompilerError;
    use crate::compiler::precompiler::{
        AstWithMetadata, PrecompilerScopeStack, precompile_ast,
    };
    use crate::libs::core::{CoreLibPointerId, get_core_lib_type};
    use crate::types::definition::TypeDefinition;
    use crate::values::core_value::CoreValue;
    use crate::values::core_values::decimal::typed_decimal::TypedDecimal;
    use crate::values::core_values::integer::integer::Integer;
    use crate::values::core_values::integer::typed_integer::{
        IntegerTypeVariant, TypedInteger,
    };
    use datex_core::values::core_values::boolean::Boolean;
    use datex_core::values::core_values::decimal::decimal::Decimal;

    /// Helper to infer the type of an expression and return it directly as Type.
    /// Panics if type inference fails or if the inferred type is not a Type.
    fn infer_get_type(expr: &mut DatexExpression) -> Type {
        infer_expression_type(
            expr,
            Rc::new(RefCell::new(AstMetadata::default())),
        )
        .map(|tc| tc.as_type())
        .expect("TypeContainer should contain a Type")
    }

    /// Parses the given source code into an AST with metadata, returning a Result.
    fn parse_and_precompile(
        src: &str,
    ) -> Result<AstWithMetadata, CompilerError> {
        let ast = parse(src).expect("Invalid expression");
        precompile_ast(
            ast,
            Rc::new(RefCell::new(AstMetadata::default())),
            &mut PrecompilerScopeStack::default(),
        )
    }

    /// Parses the given source code into an AST with metadata.
    fn parse_and_precompile_unwrap(src: &str) -> AstWithMetadata {
        parse_and_precompile(src).unwrap()
    }

    /// Parses the given source code into an AST with metadata and infers types for all expressions.
    /// Returns the metadata with all inferred types.
    /// Panics if parsing, precompilation, or type inference fails.
    fn parse_and_precompile_metadata(src: &str) -> AstMetadata {
        let cell = Rc::new(RefCell::new(AstMetadata::default()));
        {
            let ast = parse(src).expect("Invalid expression");
            let ast_with_metadata = precompile_ast(
                ast,
                cell.clone(),
                &mut PrecompilerScopeStack::default(),
            )
            .unwrap();

            let mut expr = ast_with_metadata.ast;
            infer_expression_type(
                &mut expr,
                ast_with_metadata.metadata.clone(),
            )
            .unwrap();
        }
        Rc::try_unwrap(cell)
            .expect("multiple references exist")
            .into_inner()
    }

    /// Helpers to infer the type of a type expression from source code.
    /// The source code should be a type expression, e.g. "integer/u8".
    /// The function asserts that the expression is indeed a type declaration.
    fn infer_type_container_from_str(src: &str) -> TypeContainer {
        let ast_with_metadata = parse_and_precompile_unwrap(&src);
        let mut expr = ast_with_metadata.ast;
        resolve_type_expression_type(
            match &mut expr {
                DatexExpression::TypeDeclaration { value, .. } => value,
                _ => unreachable!(),
            },
            ast_with_metadata.metadata,
        )
        .expect("Type inference failed")
    }
    fn infer_type_from_str(src: &str) -> Type {
        infer_type_container_from_str(src).as_type()
    }

    #[test]
    fn invalid_redeclaration() {
        let src = r#"
        type A = integer;
        type A = text; // redeclaration error
        "#;
        let result = parse_and_precompile(src);
        assert!(result.is_err());
        assert_matches!(
            result,
            Err(CompilerError::InvalidRedeclaration(name)) if name == "A"
        );
    }

    #[test]
    fn recursive_types() {
        let src = r#"
        type A = { b: B };
        type B = { a: A };
        "#;
        let metadata = parse_and_precompile_metadata(src);
        let var = metadata.variable_metadata(0).unwrap();
        let var_type = var.var_type.as_ref().unwrap();
        assert!(matches!(var_type, TypeContainer::TypeReference(_)));
    }

    #[test]
    fn recursive_type() {
        let src = r#"
        type LinkedList = {
            value: text,
            next: LinkedList | null
        };
        "#;
        let metadata = parse_and_precompile_metadata(src);
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
            let r#struct = bor.as_type().structural_type().unwrap();
            let fields = match r#struct {
                StructuralTypeDefinition::Struct(fields) => fields,
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
    fn assignment() {
        let src = r#"
        var a: integer = 42;
        "#;
        let metadata = parse_and_precompile_metadata(src);
        let var = metadata.variable_metadata(0).unwrap();
        assert_eq!(
            var.var_type,
            Some(get_core_lib_type(CoreLibPointerId::Integer(None)))
        );
    }

    #[test]
    fn reassignment() {
        let src = r#"
        var a: text | integer = 42;
        a = "hello";
        a = 45;
        "#;
        let metadata = parse_and_precompile_metadata(src);
        let var = metadata.variable_metadata(0).unwrap();
        assert_eq!(
            var.var_type.as_ref().map(|t| t.as_type()),
            Some(Type::union(vec![
                get_core_lib_type(CoreLibPointerId::Text),
                get_core_lib_type(CoreLibPointerId::Integer(None))
            ]))
        );
    }

    #[test]
    fn assignment_type_mismatch() {
        let src = r#"
        var a: integer = 42;
        a = "hello"; // type error
        "#;
        let ast_with_metadata = parse_and_precompile_unwrap(&src);
        let mut expr = ast_with_metadata.ast;
        let result = infer_expression_type(
            &mut expr,
            ast_with_metadata.metadata.clone(),
        );
        assert_matches!(
            result,
            Err(TypeError::AssignmentTypeMismatch {
                annotated_type,
                assigned_type
            }) if annotated_type == get_core_lib_type(CoreLibPointerId::Integer(None))
              && assigned_type.as_type() == Type::structural(StructuralTypeDefinition::Text("hello".to_string().into()))
        );
    }

    #[test]
    fn infer_type_typed_literal() {
        let inferred_type = infer_type_from_str("type X = 42u8");
        assert_eq!(
            inferred_type,
            Type::structural(StructuralTypeDefinition::TypedInteger(
                TypedInteger::U8(42)
            ))
        );

        let inferred_type = infer_type_from_str("type X = 42i32");
        assert_eq!(
            inferred_type,
            Type::structural(StructuralTypeDefinition::TypedInteger(
                TypedInteger::I32(42)
            ))
        );

        let inferred_type = infer_type_from_str("type X = 42.69f32");
        assert_eq!(
            inferred_type,
            Type::structural(StructuralTypeDefinition::TypedDecimal(
                TypedDecimal::from(42.69_f32)
            ))
        );
    }

    #[test]
    fn infer_type_simple_literal() {
        let inferred_type = infer_type_from_str("type X = 42");
        assert_eq!(
            inferred_type,
            Type::structural(StructuralTypeDefinition::Integer(Integer::from(
                42
            )))
        );

        let inferred_type = infer_type_from_str("type X = 3/4");
        assert_eq!(
            inferred_type,
            Type::structural(StructuralTypeDefinition::Decimal(
                Decimal::from_string("3/4").unwrap()
            ))
        );

        let inferred_type = infer_type_from_str("type X = true");
        assert_eq!(
            inferred_type,
            Type::structural(StructuralTypeDefinition::Boolean(Boolean(true)))
        );

        let inferred_type = infer_type_from_str("type X = false");
        assert_eq!(
            inferred_type,
            Type::structural(StructuralTypeDefinition::Boolean(Boolean(false)))
        );

        let inferred_type = infer_type_from_str(r#"type X = "hello""#);
        assert_eq!(
            inferred_type,
            Type::structural(StructuralTypeDefinition::Text(
                "hello".to_string().into()
            ))
        );
    }

    #[test]
    // TODO resolve intersection and union types properly
    // by merging the member types if one is base (one level higher) than the other
    fn infer_intersection_type_expression() {
        let inferred_type = infer_type_from_str("type X = integer/u8 & 42");
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
            infer_type_from_str("type X = integer/u8 | decimal");
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
        let inferred_type = infer_type_from_str("type X = {}");
        assert_eq!(
            inferred_type,
            Type::structural(StructuralTypeDefinition::Struct(vec![]))
        );
    }

    #[test]
    fn infer_struct_type_expression() {
        let inferred_type =
            infer_type_from_str("type X = { a: integer/u8, b: decimal }");
        assert_eq!(
            inferred_type,
            Type::structural(StructuralTypeDefinition::Struct(vec![
                (
                    "a".to_string(),
                    get_core_lib_type(CoreLibPointerId::Integer(Some(
                        IntegerTypeVariant::U8
                    )))
                ),
                (
                    "b".to_string(),
                    get_core_lib_type(CoreLibPointerId::Decimal(None))
                )
            ]))
        );
    }

    #[test]
    fn infer_core_type_expression() {
        let inferred_type =
            infer_type_container_from_str("type X = integer/u8");
        assert_eq!(
            inferred_type,
            get_core_lib_type(CoreLibPointerId::Integer(Some(
                IntegerTypeVariant::U8,
            )))
        );

        let inferred_type = infer_type_container_from_str("type X = decimal");
        assert_eq!(
            inferred_type,
            get_core_lib_type(CoreLibPointerId::Decimal(None))
        );

        let inferred_type = infer_type_container_from_str("type X = boolean");
        assert_eq!(inferred_type, get_core_lib_type(CoreLibPointerId::Boolean));

        let inferred_type = infer_type_container_from_str("type X = text");
        assert_eq!(inferred_type, get_core_lib_type(CoreLibPointerId::Text));
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
