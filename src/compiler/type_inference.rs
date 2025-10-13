use crate::ast::assignment_operation::AssignmentOperator;
use crate::ast::binary_operation::BinaryOperator;
use crate::ast::{DatexExpression, DatexExpressionData, TypeExpression};
use crate::compiler::precompiler::AstMetadata;
use crate::libs::core::{CoreLibPointerId, get_core_lib_type};
use crate::types::structural_type_definition::StructuralTypeDefinition;
use crate::types::type_container::TypeContainer;
use crate::values::core_values::r#type::Type;
use crate::values::pointer::PointerAddress;
use std::cell::RefCell;
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
pub fn infer_expression_type(
    ast: &mut DatexExpression,
    metadata: Rc<RefCell<AstMetadata>>,
) -> Result<TypeContainer, TypeError> {
    Ok(match &mut ast.data {
        DatexExpressionData::Null
        | DatexExpressionData::Boolean(_)
        | DatexExpressionData::Text(_)
        | DatexExpressionData::Decimal(_)
        | DatexExpressionData::Integer(_)
        | DatexExpressionData::TypedInteger(_)
        | DatexExpressionData::TypedDecimal(_)
        | DatexExpressionData::Endpoint(_) => {
            // TODO #446: this unwrap asserts that try_from succeeds in all cases, but this is not yet guaranteed and tested
            let value = Type::try_from(&ast.data).unwrap();
            TypeContainer::Type(value)
        }
        // composite values
        DatexExpressionData::Map(map) => {
            let entries = map
                .iter_mut()
                .map(|(k, v)| {
                    let key = infer_expression_type(k, metadata.clone())?;
                    let value = infer_expression_type(v, metadata.clone())?;
                    Ok((key, value))
                })
                .collect::<Result<Vec<(_, _)>, TypeError>>()?;
            TypeContainer::Type(Type::structural(
                StructuralTypeDefinition::Map(entries),
            ))
        }
        DatexExpressionData::List(list) => {
            let entries = list
                .iter_mut()
                .map(|v| infer_expression_type(v, metadata.clone()).unwrap())
                .collect::<Vec<_>>();
            TypeContainer::Type(Type::structural(
                StructuralTypeDefinition::List(entries),
            ))
        }
        // more complex expressions
        DatexExpressionData::BinaryOperation(operator, lhs, rhs, cached_type) => {
            infer_binary_expression_type(operator, lhs, rhs, metadata)?
        }
        DatexExpressionData::TypeExpression(type_expr) => {
            resolve_type_expression_type(type_expr, metadata)?
        }
        DatexExpressionData::TypeDeclaration {
            id,
            name: _,
            value,
            hoisted: _,
        } => {
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

            println!("Inferring type declaration id {:#?}", reference);
            // let inner_ref = reference.borrow();
            match inferred_type_def {
                TypeContainer::Type(t) => {
                    reference.borrow_mut().type_value = t;
                }
                TypeContainer::TypeReference(r) => {
                    reference.borrow_mut().type_value =
                        Type::reference(r, None);
                    // reference.swap(&r);
                }
            }

            type_def
        }
        DatexExpressionData::Variable(id, _) => {
            let var_id = *id;
            let metadata = metadata.borrow();
            metadata
                .variable_metadata(var_id)
                .expect("Variable should have variable metadata")
                .var_type
                .clone()
                .expect("Variable type should have been inferred already")
        }
        DatexExpressionData::VariableDeclaration {
            id,
            kind: _,
            name: _,
            type_annotation,
            init_expression: value,
        } => {
            // infer the type of the value expression
            let init_type = infer_expression_type(value, metadata.clone())?;

            let variable_kind = if let Some(type_annotation) = type_annotation {
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

            // TODO #447: Implement type broadened inference for example for maps
            // like var x = &mut {a: 4, y: 10} --> type Map<string, integer>
            // like var x = &mut {a: 4, y: 10} --> type {a: integer, y: integer}
            // like var x = &mut {} --> Map<unknown, unknown> -> we can set arbitrary props of any
            // var x = {a: 4, y: 10} --> type {a: 4, y: 10}

            // store type information for the variable in metadata
            let var_id = id.expect("VariableDeclaration should have an id assigned during precompilation");
            metadata
                .borrow_mut()
                .variable_metadata_mut(var_id)
                .expect("VariableDeclaration should have variable metadata")
                .var_type = Some(variable_kind.clone());

            variable_kind
        }
        DatexExpressionData::VariableAssignment(operator, id, _, value) => {
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
                op => todo!("#448 handle other assignment operators: {:?}", op),
            }
        }
        DatexExpressionData::Statements(statements) => {
            for stmt in statements.iter_mut() {
                infer_expression_type(&mut stmt.expression, metadata.clone())?;
            }
            get_core_lib_type(CoreLibPointerId::Unit)
        }
        e => panic!("Type inference not implemented for expression: {:?}", e),
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
    // This covers literals and composite types like maps and lists.
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
        TypeExpression::StructuralMap(fields) => {
            let entries = fields
                .iter_mut()
                .map(|(k, v)| {
                    let value =
                        resolve_type_expression_type(v, metadata.clone())?;
                    let key =
                        resolve_type_expression_type(k, metadata.clone())?;
                    Ok((key, value))
                })
                .collect::<Result<Vec<(_, _)>, TypeError>>()?;
            Some(StructuralTypeDefinition::Map(entries))
        }
        TypeExpression::StructuralList(members) => {
            let member_types = members
                .iter_mut()
                .map(|m| resolve_type_expression_type(m, metadata.clone()))
                .collect::<Result<Vec<_>, TypeError>>()?;
            Some(StructuralTypeDefinition::List(member_types))
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

            // TODO #449: keep the type as specific as possible here? E.g. 1 + 2 -> 3, not integer
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

        _ => todo!("#450 Undescribed by author."),
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
    use crate::libs::core::{
        CoreLibPointerId, get_core_lib_type, get_core_lib_type_reference,
    };
    use crate::references::type_reference::{
        NominalTypeDeclaration, TypeReference,
    };
    use crate::types::definition::TypeDefinition;
    use crate::values::core_value::CoreValue;
    use crate::values::core_values::decimal::typed_decimal::TypedDecimal;
    use crate::values::core_values::integer::Integer;
    use crate::values::core_values::integer::typed_integer::{
        IntegerTypeVariant, TypedInteger,
    };
    use datex_core::values::core_values::boolean::Boolean;
    use datex_core::values::core_values::decimal::Decimal;
    use crate::ast::parse_result::{DatexParseResult, InvalidDatexParseResult, ValidDatexParseResult};

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
        let parse_result = parse(src);
        match parse_result {
            DatexParseResult::Invalid(InvalidDatexParseResult { errors, .. }) => {
                panic!("Parsing failed: {:?}", errors)
            }
            DatexParseResult::Valid(valid_parse_result) => precompile_ast(
                valid_parse_result,
                Rc::new(RefCell::new(AstMetadata::default())),
                &mut PrecompilerScopeStack::default(),
            ),
        }

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
            let valid_parse_result = parse(src).unwrap();
            let ast_with_metadata = precompile_ast(
                valid_parse_result,
                cell.clone(),
                &mut PrecompilerScopeStack::default(),
            )
            .unwrap();

            let mut expr = ast_with_metadata.ast;
            infer_expression_type(
                &mut expr.as_mut().unwrap(),
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
        let ast_with_metadata = parse_and_precompile_unwrap(src);
        let mut expr = ast_with_metadata.ast;
        resolve_type_expression_type(
            match &mut expr.unwrap().data {
                DatexExpressionData::TypeDeclaration { value, .. } => value,
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
    fn nominal() {
        let src = r#"
        type A = integer;
        "#;
        let metadata = parse_and_precompile_metadata(src);
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
    fn structural() {
        let src = r#"
        typedef A = integer;
        "#;
        let metadata = parse_and_precompile_metadata(src);
        let var_a = metadata.variable_metadata(0).unwrap();
        let var_type = var_a.var_type.as_ref().unwrap();
        assert!(matches!(var_type, TypeContainer::TypeReference(_)));
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
            &mut expr.as_mut().unwrap(),
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
    // TODO #451 resolve intersection and union types properly
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
            Type::structural(StructuralTypeDefinition::Map(vec![]))
        );
    }

    #[test]
    fn infer_struct_type_expression() {
        let inferred_type =
            infer_type_from_str("type X = { a: integer/u8, b: decimal }");
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
            infer_get_type(&mut DatexExpressionData::Boolean(true).with_default_span()),
            Type::structural(StructuralTypeDefinition::Boolean(Boolean(true)))
        );

        assert_eq!(
            infer_get_type(&mut DatexExpressionData::Boolean(false).with_default_span()),
            Type::structural(StructuralTypeDefinition::Boolean(Boolean(false)))
        );

        assert_eq!(
            infer_get_type(&mut DatexExpressionData::Null.with_default_span()),
            Type::structural(StructuralTypeDefinition::Null)
        );

        assert_eq!(
            infer_get_type(&mut DatexExpressionData::Decimal(Decimal::from(1.23)).with_default_span()),
            Type::structural(StructuralTypeDefinition::Decimal(Decimal::from(
                1.23
            )))
        );

        assert_eq!(
            infer_get_type(&mut DatexExpressionData::Integer(Integer::from(42)).with_default_span()),
            Type::structural(StructuralTypeDefinition::Integer(Integer::from(
                42
            )))
        );

        assert_eq!(
            infer_get_type(&mut DatexExpressionData::List(vec![
                DatexExpressionData::Integer(Integer::from(1)).with_default_span(),
                DatexExpressionData::Integer(Integer::from(2)).with_default_span(),
                DatexExpressionData::Integer(Integer::from(3)).with_default_span()
            ]).with_default_span()),
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
            infer_get_type(&mut DatexExpressionData::Map(vec![(
                DatexExpressionData::Text("a".to_string()).with_default_span(),
                DatexExpressionData::Integer(Integer::from(1)).with_default_span()
            )]).with_default_span()),
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
    fn infer_binary_expression_types() {
        let integer = get_core_lib_type(CoreLibPointerId::Integer(None));
        let decimal = get_core_lib_type(CoreLibPointerId::Decimal(None));

        // integer - integer = integer
        let mut expr = DatexExpressionData::BinaryOperation(
            BinaryOperator::Arithmetic(ArithmeticOperator::Subtract),
            Box::new(DatexExpressionData::Integer(Integer::from(1)).with_default_span()),
            Box::new(DatexExpressionData::Integer(Integer::from(2)).with_default_span()),
            None,
        ).with_default_span();

        assert_eq!(
            infer_expression_type(
                &mut expr,
                Rc::new(RefCell::new(AstMetadata::default()))
            )
            .unwrap(),
            integer
        );

        // decimal + decimal = decimal
        let mut expr = DatexExpressionData::BinaryOperation(
            BinaryOperator::Arithmetic(ArithmeticOperator::Add),
            Box::new(DatexExpressionData::Decimal(Decimal::from(1.0)).with_default_span()),
            Box::new(DatexExpressionData::Decimal(Decimal::from(2.0)).with_default_span()),
            None,
        ).with_default_span();
        assert_eq!(
            infer_expression_type(
                &mut expr,
                Rc::new(RefCell::new(AstMetadata::default()))
            )
            .unwrap(),
            decimal
        );

        // integer + decimal = type error
        let mut expr = DatexExpressionData::BinaryOperation(
            BinaryOperator::Arithmetic(ArithmeticOperator::Add),
            Box::new(DatexExpressionData::Integer(Integer::from(1)).with_default_span()),
            Box::new(DatexExpressionData::Decimal(Decimal::from(2.0)).with_default_span()),
            None,
        ).with_default_span();
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
        let expr = DatexExpressionData::VariableDeclaration {
            id: None,
            kind: VariableKind::Const,
            name: "x".to_string(),
            type_annotation: None,
            init_expression: Box::new(DatexExpressionData::Integer(Integer::from(
                10,
            )).with_default_span()),
        }.with_default_span();

        let ast_with_metadata = precompile_ast(
            ValidDatexParseResult {
                ast: expr,
                spans: vec![0..1]
            },
            Rc::new(RefCell::new(AstMetadata::default())),
            &mut PrecompilerScopeStack::default(),
        )
        .unwrap();
        let metadata = ast_with_metadata.metadata;
        let mut expr = ast_with_metadata.ast;

        // check that the expression type is inferred correctly
        assert_eq!(
            infer_expression_type(&mut expr.as_mut().unwrap(), metadata.clone()).unwrap(),
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
}
