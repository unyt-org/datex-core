use std::{str::FromStr, vec};

use crate::ast::data::expression::DatexExpressionData;
use crate::ast::data::spanned::Spanned;
use crate::ast::data::r#type::{
    FixedSizeList, FunctionType, GenericAccess, Intersection, SliceList,
    StructuralList, StructuralMap, TypeExpression, TypeExpressionData, Union,
};
use crate::{
    ast::{
        DatexParserTrait,
        data::expression::TypeDeclaration,
        error::{
            error::{ErrorKind, ParseError},
            pattern::Pattern,
        },
        lexer::{DecimalLiteral, IntegerLiteral, Token},
        literal::literal,
        text::unescape_text,
        utils::whitespace,
    },
    references::reference::ReferenceMutability,
    values::core_values::{
        decimal::{Decimal, typed_decimal::TypedDecimal},
        endpoint::Endpoint,
        integer::{Integer, typed_integer::TypedInteger},
    },
};
use chumsky::{
    IterParser, Parser,
    prelude::{choice, just, recursive},
    select,
};

pub fn integer<'a>() -> impl DatexParserTrait<'a, TypeExpressionData> {
    select! {
        Token::DecimalIntegerLiteral(IntegerLiteral { value, variant }) => {
            match variant {
                Some(var) => TypedInteger::from_string_with_variant(&value, var)
                    .map(TypeExpressionData::TypedInteger),
                None => Integer::from_string(&value)
                    .map(TypeExpressionData::Integer),
            }
        },
        Token::BinaryIntegerLiteral(IntegerLiteral { value, variant }) => {
            match variant {
                Some(var) => TypedInteger::from_string_radix_with_variant(&value[2..], 2, var)
                    .map(TypeExpressionData::TypedInteger),
                None => Integer::from_string_radix(&value[2..], 2)
                    .map(TypeExpressionData::Integer),
            }
        },
        Token::HexadecimalIntegerLiteral(IntegerLiteral { value, variant }) => {
            match variant {
                Some(var) => TypedInteger::from_string_radix_with_variant(&value[2..], 16, var)
                    .map(TypeExpressionData::TypedInteger),
                None => Integer::from_string_radix(&value[2..], 16)
                    .map(TypeExpressionData::Integer),
            }
        },
        Token::OctalIntegerLiteral(IntegerLiteral { value, variant }) => {
            match variant {
                Some(var) => TypedInteger::from_string_radix_with_variant(&value[2..], 8, var)
                    .map(TypeExpressionData::TypedInteger),
                None => Integer::from_string_radix(&value[2..], 8)
                    .map(TypeExpressionData::Integer),
            }
        },
    }.try_map(|res, _| {
		res.map_err(|e| ParseError::new(ErrorKind::NumberParseError(e)))
	})
}

pub fn integer_to_usize(i: &TypeExpressionData) -> Option<usize> {
    match i {
        TypeExpressionData::Integer(v) => v.as_usize(),
        TypeExpressionData::TypedInteger(v) => v.as_usize(),
        _ => None,
    }
}

pub fn decimal<'a>() -> impl DatexParserTrait<'a, TypeExpressionData> {
    select! {
        Token::DecimalLiteral(DecimalLiteral { value, variant }) => {
            match variant {
                Some(var) => TypedDecimal::from_string_and_variant_in_range(&value, var).map(TypeExpressionData::TypedDecimal),
                None => Decimal::from_string(&value).map(TypeExpressionData::Decimal)
            }
        },
        Token::FractionLiteral(s) => Decimal::from_string(&s).map(TypeExpressionData::Decimal),
    }.try_map(|res, _| {
		res.map_err(|e| ParseError::new(ErrorKind::NumberParseError(e)))
	})
}

pub fn r#type<'a>() -> impl DatexParserTrait<'a, TypeExpression> {
    recursive(|ty| {
        let paren_group = ty.clone().delimited_by(
            just(Token::LeftParen).padded_by(whitespace()),
            just(Token::RightParen).padded_by(whitespace()),
        );

        // Parse a type reference, e.g. `integer`, `text`, `User` etc.
        let type_reference = choice((
            select! { Token::Identifier(s) => s }
                .then(
                    just(Token::Slash)
                        .ignore_then(select! { Token::Identifier(s) => s })
                        .or_not(),
                )
                .map_with(|(base, sub): (String, Option<String>), e| match sub
                    .as_deref()
                {
                    None => {
                        TypeExpressionData::Literal(base).with_span(e.span())
                    }
                    Some(variant) => TypeExpressionData::Literal(format!(
                        "{}/{}",
                        base, variant
                    ))
                    .with_span(e.span()),
                }),
            just(Token::Null)
                .map_with(|_, e| TypeExpressionData::Null.with_span(e.span())),
        ));

        let literal =
            choice((
				select! {
					Token::StringLiteral(s) => TypeExpressionData::Text(unescape_text(&s)),
				},
				select! {
					Token::True => TypeExpressionData::Boolean(true),
					Token::False => TypeExpressionData::Boolean(false),
				},
				select! {
					Token::Endpoint(s) =>
						Endpoint::from_str(s.as_str())
				}.try_map(|res, _| {
					res.map(TypeExpressionData::Endpoint)
						.map_err(|e| ParseError::new(ErrorKind::InvalidEndpoint(e)))
				}),
				integer(),
				decimal()
			))
			.padded_by(whitespace())
            .map_with(|data, e| data.with_span(e.span()));

        let list_inline = ty
            .clone()
            .padded_by(whitespace())
            .separated_by(just(Token::Comma))
            .allow_trailing()
            .collect()
            .delimited_by(
                just(Token::LeftBracket).padded_by(whitespace()),
                just(Token::RightBracket).padded_by(whitespace()),
            )
            .map(|elems: Vec<TypeExpression>| {
                TypeExpressionData::StructuralList(StructuralList(elems))
                    .with_default_span() // FIXME span handling
            });

        let list_fixed_inline = ty
            .clone()
            .then_ignore(just(Token::Semicolon).padded_by(whitespace()))
            .then(integer().clone())
            .delimited_by(
                just(Token::LeftBracket).padded_by(whitespace()),
                just(Token::RightBracket).padded_by(whitespace()),
            )
            .try_map(|(t, size), _| {
                if let Some(n) = integer_to_usize(&size)
                    && n > 0
                {
                    Ok(TypeExpressionData::FixedSizeList(FixedSizeList {
                        r#type: Box::new(t),
                        size: n,
                    })
                    .with_default_span())
                } else {
                    Err(ParseError::new(ErrorKind::InvalidListSize(format!(
                        "{size:?}"
                    ))))
                }
            });

        let key_ident =
            select! { Token::Identifier(k) => k }.padded_by(whitespace());
        // let r#struct = key_ident
        //     .clone()
        //     .then_ignore(just(Token::Colon))
        //     .padded_by(whitespace())
        //     .then(ty.clone())
        //     .padded_by(whitespace())
        //     .separated_by(just(Token::Comma))
        //     .allow_trailing()
        //     .collect()
        //     .delimited_by(
        //         just(Token::LeftCurly).padded_by(whitespace()),
        //         just(Token::RightCurly).padded_by(whitespace()),
        //     )
        //     .map(|fields: Vec<(String, TypeContainer)>| {
        //         Type::r#struct(fields).as_type_container()
        //     });
        let struct_field = select! {
           Token::Identifier(k) => TypeExpressionData::Text(k),
           Token::StringLiteral(k) => TypeExpressionData::Text(unescape_text(&k)),
        }
        .then(just(Token::Placeholder).or_not())
        .then_ignore(just(Token::Colon).padded_by(whitespace()))
        .then(ty.clone())
        .map(|((name, opt), typ)| {
            if opt.is_some() {
                (name, TypeExpressionData::Union(Union(vec![typ, TypeExpressionData::Null.with_default_span()])).with_default_span())
            } else {
                (name, typ)
            }
        });

        let structural_map = struct_field
            .separated_by(just(Token::Comma).padded_by(whitespace()))
            .allow_trailing()
            .collect()
            .delimited_by(
                just(Token::LeftCurly).padded_by(whitespace()),
                just(Token::RightCurly).padded_by(whitespace()),
            )
            .map(|fields: Vec<(TypeExpression, TypeExpression)>| {
                TypeExpressionData::StructuralMap(StructuralMap(fields))
                    .with_default_span()
            });

        let generic = select! { Token::Identifier(name) => name }
            .then(
                ty.clone()
                    .separated_by(just(Token::Comma).padded_by(whitespace()))
                    .allow_trailing()
                    .collect()
                    .padded_by(whitespace())
                    .delimited_by(
                        just(Token::LeftAngle),
                        just(Token::RightAngle),
                    ),
            )
            .map(|(name, args): (String, Vec<TypeExpression>)| {
                TypeExpressionData::GenericAccess(GenericAccess {
                    base: name,
                    access: args,
                })
            });

        let func = key_ident
            .then_ignore(just(Token::Colon).padded_by(whitespace()))
            .then(ty.clone())
            .separated_by(just(Token::Comma))
            .allow_trailing()
            .collect()
            .delimited_by(
                just(Token::LeftParen).padded_by(whitespace()),
                just(Token::RightParen).padded_by(whitespace()),
            )
            .then_ignore(just(Token::Arrow).padded_by(whitespace()))
            .then(ty.clone())
            .map_with(
                |(params, ret): (
                    Vec<(String, TypeExpression)>,
                    TypeExpression,
                ),
                 e| {
                    TypeExpressionData::Function(FunctionType {
                        parameters: params,
                        return_type: Box::new(ret),
                    })
                    .with_span(e.span())
                },
            );

        let reference = just(Token::Ampersand)
            .ignore_then(just(Token::Mutable).or(just(Token::Final)).or_not())
            .then_ignore(whitespace())
            .then(ty.clone())
            .map_with(
                |(maybe_mut, inner): (Option<Token>, TypeExpression), e| {
                    let mutability = match maybe_mut {
                        Some(Token::Mutable) => ReferenceMutability::Mutable,
                        Some(Token::Final) => ReferenceMutability::Final,
                        None => ReferenceMutability::Immutable,
                        _ => unreachable!(),
                    };
                    match mutability {
                        ReferenceMutability::Mutable => {
                            TypeExpressionData::RefMut(Box::new(inner))
                        }
                        ReferenceMutability::Immutable => {
                            TypeExpressionData::Ref(Box::new(inner))
                        }
                        ReferenceMutability::Final => {
                            TypeExpressionData::RefFinal(Box::new(inner))
                        }
                    }
                    .with_span(e.span())
                },
            );

        let base = choice((
            reference.clone(),
            func.clone(),
            literal.clone(),
            list_inline.clone(),
            list_fixed_inline.clone(),
            structural_map.clone(),
            generic.clone(),
            paren_group.clone(),
            type_reference.clone(),
        ));

        // let field_access = base
        //     .clone()
        //     .then(
        //         just(Token::Dot)
        //             .ignore_then(select! { Token::Identifier(name) => name })
        //             .repeated()
        //             .collect(),
        //     )
        //     .map(|(root, fields): (TypeContainer, Vec<String>)| {
        //         fields.into_iter().fold(root, |acc, field| {
        //             Type::field_access(acc, field).as_type_container()
        //         })
        //     });

        // let index_access = base.clone().then(
        // 	just(Token::LeftBracket)
        // 		.ignore_then(ty.clone())
        // 		.then_ignore(just(Token::RightBracket))
        // 		.repeated()
        // 		.collect(),
        // ).map(|(root, indices): (TypeContainer, Vec<TypeContainer>)| {
        // 	indices.into_iter().fold(root, |acc, idx| {
        // 		Type::index_access(acc, idx).as_type_container()
        // 	})
        // });

        // parse zero-or-more postfix `[]`
        // let optional_postfix_array = base
        //     .then(
        //         just(Token::LeftBracket)
        //             .ignore_then(just(Token::RightBracket))
        //             .repeated()
        //             .count(),
        //     )
        //     .map(|(base_tc, count): (TypeExpression, usize)| {
        //         let mut t = base_tc;
        //         for _ in 0..count {
        //             t = TypeExpression::Array(vec![t]);
        //         }
        //         t
        //     });
        let postfix_array = just(Token::LeftBracket).ignore_then(choice((
            // Slice: []
            just(Token::RightBracket).to(None),
            // Fixed-size: [10]
            integer().then_ignore(just(Token::RightBracket)).map(Some),
            // Fixed-size alternative: [; 10]
            just(Token::Semicolon)
                .padded_by(whitespace())
                .ignore_then(integer().padded_by(whitespace()))
                .then_ignore(just(Token::RightBracket))
                .map(Some),
        )));

        // TODO #365: consider and update accordingly
        let postfix_array = just(Token::LeftBracket).ignore_then(choice((
            // Slice: []
            just(Token::RightBracket).to(None),
            // Fixed size: [10]
            integer().then_ignore(just(Token::RightBracket)).map(Some),
        )));

        let array_postfix = base
            .then(postfix_array.repeated().collect::<Vec<_>>())
            .try_map_with(|(mut t, arrs), e| {
                for arr in arrs {
                    t = match arr {
                        None => TypeExpressionData::SliceList(SliceList(
                            Box::new(t),
                        ))
                        .with_span(e.span()),
                        Some(n) => match integer_to_usize(&n) {
                            Some(size) if size > 0 => {
                                TypeExpressionData::FixedSizeList(
                                    FixedSizeList {
                                        r#type: Box::new(t),
                                        size,
                                    },
                                )
                                .with_span(e.span())
                            }
                            _ => {
                                return Err(ParseError::new(
                                    ErrorKind::InvalidListSize(format!(
                                        "{n:?}"
                                    )),
                                ));
                            }
                        },
                    };
                }
                Ok(t)
            });

        let intersection = array_postfix
            .clone()
            .then(
                // parse zero-or-more `& <postfix_array>`
                just(Token::Ampersand)
                    .padded_by(whitespace())
                    .ignore_then(array_postfix.clone())
                    .repeated()
                    .collect(),
            )
            .map_with(
                |(first, mut rest): (TypeExpression, Vec<TypeExpression>),
                 e| {
                    if rest.is_empty() {
                        return first;
                    }
                    rest.insert(0, first);
                    TypeExpressionData::Intersection(Intersection(rest))
                        .with_span(e.span())
                },
            );

        intersection
            .clone()
            .then(
                just(Token::Pipe)
                    .padded_by(whitespace())
                    .ignore_then(intersection.clone())
                    .repeated()
                    .collect(),
            )
            .map_with(
                |(first, mut rest): (TypeExpression, Vec<TypeExpression>),
                 e| {
                    if rest.is_empty() {
                        return first;
                    }
                    rest.insert(0, first);
                    TypeExpressionData::Union(Union(rest)).with_span(e.span())
                },
            )
    })
}

pub fn nominal_type_declaration<'a>() -> impl DatexParserTrait<'a> {
    let generic = just(Token::LeftAngle)
        .ignore_then(literal())
        .then_ignore(just(Token::RightAngle))
        .or_not();

    let name = select! { Token::Identifier(name) => name }
        .then(
            just(Token::Slash)
                .ignore_then(select! { Token::Identifier(name) => name })
                .or_not(),
        )
        .map(|(base, opt_suffix)| match opt_suffix {
            Some(suffix) => format!("{}/{}", base, suffix),
            None => base,
        });

    just(Token::Identifier("type".to_string()))
        .padded_by(whitespace())
        .ignore_then(name)
        .then(generic)
        .then_ignore(just(Token::Assign).padded_by(whitespace()))
        .then(r#type())
        .padded_by(whitespace())
        .map_with(|((name, generic), expr), e| {
            DatexExpressionData::TypeDeclaration(TypeDeclaration {
                id: None,
                name: name.to_string(),
                value: expr,
                hoisted: false,
            })
            .with_span(e.span())
        })
        .labelled(Pattern::Declaration)
        .as_context()
}

pub fn structural_type_definition<'a>() -> impl DatexParserTrait<'a> {
    just(Token::Identifier("typedef".to_string()))
        .padded_by(whitespace())
        .ignore_then(select! { Token::Identifier(name) => name })
        .then_ignore(just(Token::Assign).padded_by(whitespace()))
        .then(r#type())
        .map_with(|(name, expr), e| {
            DatexExpressionData::TypeDeclaration(TypeDeclaration {
                id: None,
                name: name.to_string(),
                value: expr,
                hoisted: false,
            })
            .with_span(e.span())
        })
        .labelled(Pattern::Declaration)
        .as_context()
}

pub fn type_declaration<'a>() -> impl DatexParserTrait<'a> {
    choice((nominal_type_declaration(), structural_type_definition()))
}

// structural type expression
pub fn type_expression<'a>() -> impl DatexParserTrait<'a> {
    just(Token::Identifier("type".to_string()))
        .padded_by(whitespace())
        .then_ignore(just(Token::LeftParen).padded_by(whitespace()))
        .ignore_then(r#type())
        .padded_by(whitespace())
        .then_ignore(just(Token::RightParen).padded_by(whitespace()))
        .map_with(|expr, e| DatexExpressionData::Type(expr).with_span(e.span()))
}

#[cfg(test)]
mod tests {
    use crate::ast::{DatexParseResult, error::src::SrcId, parse};

    use super::*;
    use crate::ast::data::expression::{
        DatexExpression, DatexExpressionData, Statements,
    };
    use crate::ast::parse_result::{
        InvalidDatexParseResult, ValidDatexParseResult,
    };
    use std::{io, str::FromStr};

    fn parse_unwrap(src: &str) -> DatexExpressionData {
        let src_id = SrcId::test();
        let res = parse(src);
        match res {
            DatexParseResult::Invalid(InvalidDatexParseResult {
                errors,
                ..
            }) => {
                errors.iter().for_each(|e| {
                    let cache = ariadne::sources(vec![(src_id, src)]);
                    e.clone().write(cache, io::stdout());
                });
                panic!("Parsing errors found");
            }
            DatexParseResult::Valid(ValidDatexParseResult { ast, .. }) => {
                ast.data
            }
        }
    }
    fn parse_type_unwrap(src: &str) -> TypeExpressionData {
        let value = parse_unwrap(format!("type T = {}", src).as_str());
        if let DatexExpressionData::TypeDeclaration(TypeDeclaration {
            value,
            ..
        }) = value
        {
            value.data
        } else if let DatexExpressionData::Statements(Statements {
            statements,
            ..
        }) = &value
            && statements.len() == 1
        {
            match &statements[0].data {
                DatexExpressionData::TypeDeclaration(TypeDeclaration {
                    value,
                    ..
                }) => value.data.clone(),
                _ => {
                    panic!("Expected TypeDeclaration, got {:?}", statements[0])
                }
            }
        } else {
            panic!("Expected TypeDeclaration, got {:?}", value);
        }
    }

    #[test]
    fn literal() {
        let src = "integer/u16";
        let val = parse_type_unwrap(src);
        assert_eq!(val, TypeExpressionData::Literal("integer/u16".to_owned()));
    }

    #[test]
    fn structural_map() {
        let src = r#"
			{
				"name": text | null,
				age: integer | text
			}
		"#;
        let val = parse_type_unwrap(src);
        assert_eq!(
            val,
            TypeExpressionData::StructuralMap(StructuralMap(vec![
                (
                    TypeExpressionData::Text("name".to_string())
                        .with_default_span(),
                    TypeExpressionData::Union(Union(vec![
                        TypeExpressionData::Literal("text".to_owned())
                            .with_default_span(),
                        TypeExpressionData::Null.with_default_span()
                    ]))
                    .with_default_span()
                ),
                (
                    TypeExpressionData::Text("age".to_string())
                        .with_default_span(),
                    TypeExpressionData::Union(Union(vec![
                        TypeExpressionData::Literal("integer".to_owned())
                            .with_default_span(),
                        TypeExpressionData::Literal("text".to_owned())
                            .with_default_span()
                    ]))
                    .with_default_span()
                )
            ]))
        );

        let src = r#"
            {
                name?: text,
                friends: List<&text>
            };
        "#;
        let val = parse_type_unwrap(src);
        assert_eq!(
            val,
            TypeExpressionData::StructuralMap(StructuralMap(vec![
                (
                    TypeExpressionData::Text("name".to_string())
                        .with_default_span(),
                    TypeExpressionData::Union(Union(vec![
                        TypeExpressionData::Literal("text".to_owned())
                            .with_default_span(),
                        TypeExpressionData::Null.with_default_span()
                    ]))
                    .with_default_span()
                ),
                (
                    TypeExpressionData::Text("friends".to_string())
                        .with_default_span(),
                    TypeExpressionData::GenericAccess(GenericAccess {
                        base: "List".to_owned(),
                        access: vec![
                            TypeExpressionData::Ref(Box::new(
                                TypeExpressionData::Literal("text".to_owned())
                                    .with_default_span()
                            ))
                            .with_default_span()
                        ]
                    })
                    .with_default_span()
                ),
            ]))
        );

        let src = r#"
        	{
        		name: text,
        		friends: List<&text>
        	}
        "#;
        let val = parse_type_unwrap(src);
        assert_eq!(
            val,
            TypeExpressionData::StructuralMap(StructuralMap(vec![
                (
                    TypeExpressionData::Text("name".to_string())
                        .with_default_span(),
                    TypeExpressionData::Literal("text".to_owned())
                        .with_default_span()
                ),
                (
                    TypeExpressionData::Text("friends".to_string())
                        .with_default_span(),
                    TypeExpressionData::GenericAccess(GenericAccess {
                        base: "List".to_owned(),
                        access: vec![
                            TypeExpressionData::Ref(Box::new(
                                TypeExpressionData::Literal("text".to_owned())
                                    .with_default_span()
                            ))
                            .with_default_span()
                        ]
                    })
                    .with_default_span()
                ),
            ]))
        );

        let src = r#"
            {
                name: text,
                age: &mut text
            }
        "#;
        let val = parse_type_unwrap(src);
        assert_eq!(
            val,
            TypeExpressionData::StructuralMap(StructuralMap(vec![
                (
                    TypeExpressionData::Text("name".to_string())
                        .with_default_span(),
                    TypeExpressionData::Literal("text".to_owned())
                        .with_default_span()
                ),
                (
                    TypeExpressionData::Text("age".to_string())
                        .with_default_span(),
                    TypeExpressionData::RefMut(Box::new(
                        TypeExpressionData::Literal("text".to_owned())
                            .with_default_span()
                    ))
                    .with_default_span()
                ),
            ]))
        );
    }

    #[test]
    fn union_flat() {
        let src = r#""hello world" | 42"#;
        let val = parse_type_unwrap(src);
        assert_eq!(
            val,
            TypeExpressionData::Union(Union(vec![
                TypeExpressionData::Text("hello world".to_owned())
                    .with_default_span(),
                TypeExpressionData::Integer(Integer::from(42))
                    .with_default_span(),
            ]))
        );

        let src = "1 | 2 | 3 | 4";
        let val = parse_type_unwrap(src);
        assert_eq!(
            val,
            TypeExpressionData::Union(Union(vec![
                TypeExpressionData::Integer(Integer::from(1))
                    .with_default_span(),
                TypeExpressionData::Integer(Integer::from(2))
                    .with_default_span(),
                TypeExpressionData::Integer(Integer::from(3))
                    .with_default_span(),
                TypeExpressionData::Integer(Integer::from(4))
                    .with_default_span(),
            ]))
        );

        let src = "@jonas | @bene";
        let val = parse_type_unwrap(src);
        assert_eq!(
            val,
            TypeExpressionData::Union(Union(vec![
                TypeExpressionData::Endpoint(
                    Endpoint::from_str("@jonas").unwrap()
                )
                .with_default_span(),
                TypeExpressionData::Endpoint(
                    Endpoint::from_str("@bene").unwrap()
                )
                .with_default_span(),
            ]))
        );
    }

    #[test]
    fn union_nested() {
        let src = "(1 | 2) | 3 | 4";
        let val = parse_type_unwrap(src);
        assert_eq!(
            val,
            TypeExpressionData::Union(Union(vec![
                TypeExpressionData::Union(Union(vec![
                    TypeExpressionData::Integer(Integer::from(1))
                        .with_default_span(),
                    TypeExpressionData::Integer(Integer::from(2))
                        .with_default_span(),
                ]))
                .with_default_span(),
                TypeExpressionData::Integer(Integer::from(3))
                    .with_default_span(),
                TypeExpressionData::Integer(Integer::from(4))
                    .with_default_span(),
            ]))
        );
    }

    #[test]
    fn union_and_intersection() {
        let src = "1 | (2 & 3) | 4";
        let val = parse_type_unwrap(src);
        assert_eq!(
            val,
            TypeExpressionData::Union(Union(vec![
                TypeExpressionData::Integer(Integer::from(1))
                    .with_default_span(),
                TypeExpressionData::Intersection(Intersection(vec![
                    TypeExpressionData::Integer(Integer::from(2))
                        .with_default_span(),
                    TypeExpressionData::Integer(Integer::from(3))
                        .with_default_span(),
                ]))
                .with_default_span(),
                TypeExpressionData::Integer(Integer::from(4))
                    .with_default_span(),
            ]))
        );

        let src = "(1 | 2) & 3 & 4";
        let val = parse_type_unwrap(src);
        assert_eq!(
            val,
            TypeExpressionData::Intersection(Intersection(vec![
                TypeExpressionData::Union(Union(vec![
                    TypeExpressionData::Integer(Integer::from(1))
                        .with_default_span(),
                    TypeExpressionData::Integer(Integer::from(2))
                        .with_default_span(),
                ]))
                .with_default_span(),
                TypeExpressionData::Integer(Integer::from(3))
                    .with_default_span(),
                TypeExpressionData::Integer(Integer::from(4))
                    .with_default_span(),
            ]))
        );
    }

    #[test]
    fn structural_list() {
        let src = "[1, 2, 3, 4]";
        let val = parse_type_unwrap(src);
        assert_eq!(
            val,
            TypeExpressionData::StructuralList(StructuralList(vec![
                TypeExpressionData::Integer(Integer::from(1))
                    .with_default_span(),
                TypeExpressionData::Integer(Integer::from(2))
                    .with_default_span(),
                TypeExpressionData::Integer(Integer::from(3))
                    .with_default_span(),
                TypeExpressionData::Integer(Integer::from(4))
                    .with_default_span(),
            ]))
        );

        let src = "[1,2,text]";
        let val = parse_type_unwrap(src);
        assert_eq!(
            val,
            TypeExpressionData::StructuralList(StructuralList(vec![
                TypeExpressionData::Integer(Integer::from(1))
                    .with_default_span(),
                TypeExpressionData::Integer(Integer::from(2))
                    .with_default_span(),
                TypeExpressionData::Literal("text".to_owned())
                    .with_default_span(),
            ]))
        );

        let src = "[integer|text]";
        let val = parse_type_unwrap(src);
        assert_eq!(
            val,
            TypeExpressionData::StructuralList(StructuralList(vec![
                TypeExpressionData::Union(Union(vec![
                    TypeExpressionData::Literal("integer".to_owned())
                        .with_default_span(),
                    TypeExpressionData::Literal("text".to_owned())
                        .with_default_span(),
                ]))
                .with_default_span(),
            ]))
        );
    }

    #[test]
    fn fixed_sized_list_1() {
        let src = "integer[10]";
        let val = parse_type_unwrap(src);
        assert_eq!(
            val,
            TypeExpressionData::FixedSizeList(FixedSizeList {
                r#type: Box::new(
                    TypeExpressionData::Literal("integer".to_owned())
                        .with_default_span()
                ),
                size: 10
            })
        );

        let src = "(integer | string)[10]";
        let val = parse_type_unwrap(src);
        assert_eq!(
            val,
            TypeExpressionData::FixedSizeList(FixedSizeList {
                r#type: Box::new(
                    TypeExpressionData::Union(Union(vec![
                        TypeExpressionData::Literal("integer".to_owned())
                            .with_default_span(),
                        TypeExpressionData::Literal("string".to_owned())
                            .with_default_span(),
                    ]))
                    .with_default_span()
                ),
                size: 10
            })
        );
    }

    #[test]
    fn fixed_sized_list_2() {
        let src = "[text; 4]";
        let val = parse_type_unwrap(src);
        assert_eq!(
            val,
            TypeExpressionData::FixedSizeList(
                Box::new(TypeExpressionData::Literal("text".to_owned()))
                    .with_default_span(),
                4
            )
        );

        let src = "[text;  42]";
        let val = parse_type_unwrap(src);
        assert_eq!(
            val,
            TypeExpressionData::FixedSizeList(
                Box::new(TypeExpressionData::Literal("text".to_owned()))
                    .with_default_span(),
                42
            )
        );

        let src = "[text;10]";
        let val = parse_type_unwrap(src);
        assert_eq!(
            val,
            TypeExpressionData::FixedSizeList(
                Box::new(TypeExpressionData::Literal("text".to_owned()))
                    .with_default_span(),
                10
            )
        );
    }

    #[test]
    fn slice_list() {
        let src = "text[]";
        let val = parse_type_unwrap(src);
        assert_eq!(
            val,
            TypeExpressionData::SliceList(Box::new(
                TypeExpressionData::Literal("text".to_owned())
                    .with_default_span()
            ))
        );

        let src = "integer[][][]";
        let val = parse_type_unwrap(src);
        assert_eq!(
            val,
            TypeExpressionData::SliceList(Box::new(
                TypeExpressionData::SliceList(Box::new(
                    TypeExpressionData::SliceList(Box::new(
                        TypeExpressionData::Literal("integer".to_owned())
                            .with_default_span()
                    ))
                ))
            ))
        );
    }

    #[test]
    fn generic_1() {
        let src = "List<integer>";
        let val = parse_type_unwrap(src);
        assert_eq!(
            val,
            TypeExpressionData::GenericAccess(
                "List".to_owned(),
                vec![
                    TypeExpressionData::Literal("integer".to_owned())
                        .with_default_span()
                ],
            )
        );

        let src = "List<integer | text>";
        let val = parse_type_unwrap(src);
        assert_eq!(
            val,
            TypeExpressionData::GenericAccess(
                "List".to_owned(),
                vec![TypeExpressionData::Union(vec![
                    TypeExpressionData::Literal("integer".to_owned())
                        .with_default_span(),
                    TypeExpressionData::Literal("text".to_owned())
                        .with_default_span(),
                ]),],
            )
        );
    }

    #[test]
    fn generic_2() {
        let src = "Map<text, integer>";
        let val = parse_type_unwrap(src);
        assert_eq!(
            val,
            TypeExpressionData::GenericAccess(
                "Map".to_owned(),
                vec![
                    TypeExpressionData::Literal("text".to_owned())
                        .with_default_span(),
                    TypeExpressionData::Literal("integer".to_owned())
                        .with_default_span(),
                ],
            )
        );
    }

    #[test]
    fn generic_type() {
        let src = "User<text, integer>";
        let val = parse_type_unwrap(src);
        assert_eq!(
            val,
            TypeExpressionData::GenericAccess(
                "User".to_owned(),
                vec![
                    TypeExpressionData::Literal("text".to_owned())
                        .with_default_span(),
                    TypeExpressionData::Literal("integer".to_owned())
                        .with_default_span(),
                ],
            )
        );

        let src = "User<text | integer>";
        let val = parse_type_unwrap(src);
        assert_eq!(
            val,
            TypeExpressionData::GenericAccess(
                "User".to_owned(),
                vec![TypeExpressionData::Union(vec![
                    TypeExpressionData::Literal("text".to_owned())
                        .with_default_span(),
                    TypeExpressionData::Literal("integer".to_owned())
                        .with_default_span(),
                ]),],
            )
        );
    }

    #[test]
    fn function() {
        let src = "(x: text, y: text | 4.5) -> text | 52";
        let val = parse_type_unwrap(src);
        assert_eq!(
            val,
            TypeExpressionData::Function {
                parameters: vec![
                    (
                        "x".to_string(),
                        TypeExpressionData::Literal("text".to_owned())
                            .with_default_span()
                    ),
                    (
                        "y".to_string(),
                        TypeExpressionData::Union(vec![
                            TypeExpressionData::Literal("text".to_owned())
                                .with_default_span(),
                            TypeExpressionData::Decimal(
                                Decimal::from_string("4.5")
                                    .unwrap()
                                    .with_default_span()
                            )
                        ])
                    )
                ],
                return_type: Box::new(TypeExpressionData::Union(vec![
                    TypeExpressionData::Literal("text".to_owned())
                        .with_default_span(),
                    TypeExpressionData::Integer(Integer::from(52))
                        .with_default_span()
                ])),
            }
        );

        let src = "(x: &mut text, y: text | 4.5) -> text | 52";
        let val = parse_type_unwrap(src);
        assert_eq!(
            val,
            TypeExpressionData::Function {
                parameters: vec![
                    (
                        "x".to_string(),
                        TypeExpressionData::RefMut(Box::new(
                            TypeExpressionData::Literal("text".to_owned())
                                .with_default_span()
                        ))
                        .with_default_span()
                    ),
                    (
                        "y".to_string(),
                        TypeExpressionData::Union(vec![
                            TypeExpressionData::Literal("text".to_owned())
                                .with_default_span(),
                            TypeExpressionData::Decimal(
                                Decimal::from_string("4.5").unwrap()
                            )
                            .with_default_span()
                        ])
                    )
                ],
                return_type: Box::new(TypeExpressionData::Union(vec![
                    TypeExpressionData::Literal("text".to_owned())
                        .with_default_span(),
                    TypeExpressionData::Integer(Integer::from(52))
                        .with_default_span()
                ])),
            }
        );
    }

    #[test]
    fn mix_1() {
        let src = "&[&mut text, &mut integer/u8]";
        let val = parse_type_unwrap(src);
        assert_eq!(
            val,
            TypeExpressionData::Ref(Box::new(
                TypeExpressionData::StructuralList(vec![
                    TypeExpressionData::RefMut(Box::new(
                        TypeExpressionData::Literal("text".to_owned())
                            .with_default_span()
                    ))
                    .with_default_span(),
                    TypeExpressionData::RefMut(Box::new(
                        TypeExpressionData::Literal("integer/u8".to_owned())
                            .with_default_span()
                    ))
                    .with_default_span(),
                ])
            ))
        );
    }
}
