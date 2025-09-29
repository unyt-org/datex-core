use std::{str::FromStr, vec};

use chumsky::{
    IterParser, Parser,
    prelude::{choice, just, recursive},
    select,
};

use crate::{
    ast::{
        DatexExpression, DatexParserTrait, TypeExpression,
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
        decimal::{decimal::Decimal, typed_decimal::TypedDecimal},
        endpoint::Endpoint,
        integer::{integer::Integer, typed_integer::TypedInteger},
    },
};

pub fn integer<'a>() -> impl DatexParserTrait<'a, TypeExpression> {
    select! {
        Token::DecimalIntegerLiteral(IntegerLiteral { value, variant }) => {
            match variant {
                Some(var) => TypedInteger::from_string_with_variant(&value, var)
                    .map(TypeExpression::TypedInteger),
                None => Integer::from_string(&value)
                    .map(TypeExpression::Integer),
            }
        },
        Token::BinaryIntegerLiteral(IntegerLiteral { value, variant }) => {
            match variant {
                Some(var) => TypedInteger::from_string_radix_with_variant(&value[2..], 2, var)
                    .map(TypeExpression::TypedInteger),
                None => Integer::from_string_radix(&value[2..], 2)
                    .map(TypeExpression::Integer),
            }
        },
        Token::HexadecimalIntegerLiteral(IntegerLiteral { value, variant }) => {
            match variant {
                Some(var) => TypedInteger::from_string_radix_with_variant(&value[2..], 16, var)
                    .map(TypeExpression::TypedInteger),
                None => Integer::from_string_radix(&value[2..], 16)
                    .map(TypeExpression::Integer),
            }
        },
        Token::OctalIntegerLiteral(IntegerLiteral { value, variant }) => {
            match variant {
                Some(var) => TypedInteger::from_string_radix_with_variant(&value[2..], 8, var)
                    .map(TypeExpression::TypedInteger),
                None => Integer::from_string_radix(&value[2..], 8)
                    .map(TypeExpression::Integer),
            }
        },
    }.try_map(|res, _| {
		res.map_err(|e| ParseError::new(ErrorKind::NumberParseError(e)))
	})
}

pub fn integer_to_usize(i: &TypeExpression) -> Option<usize> {
    match i {
        TypeExpression::Integer(v) => v.as_usize(),
        TypeExpression::TypedInteger(v) => v.as_usize(),
        _ => None,
    }
}

pub fn decimal<'a>() -> impl DatexParserTrait<'a, TypeExpression> {
    select! {
        Token::DecimalLiteral(DecimalLiteral { value, variant }) => {
            match variant {
                Some(var) => TypedDecimal::from_string_and_variant_in_range(&value, var).map(TypeExpression::TypedDecimal),
                None => Decimal::from_string(&value).map(TypeExpression::Decimal)
            }
        },
        Token::FractionLiteral(s) => Decimal::from_string(&s).map(TypeExpression::Decimal),
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
                .map(|(base, sub): (String, Option<String>)| {
                    match sub.as_deref() {
                        None => TypeExpression::Literal(base),
                        Some(variant) => TypeExpression::Literal(format!(
                            "{}/{}",
                            base, variant
                        )),
                    }
                }),
            just(Token::Null).map(|_| TypeExpression::Null),
        ));

        let literal =
            choice((
				select! {
					Token::StringLiteral(s) => TypeExpression::Text(unescape_text(&s)),
				},
				select! {
					Token::True => TypeExpression::Boolean(true),
					Token::False => TypeExpression::Boolean(false),
				},
				select! {
					Token::Endpoint(s) =>
						Endpoint::from_str(s.as_str())
				}.try_map(|res, _| {
					res.map(TypeExpression::Endpoint)
						.map_err(|e| ParseError::new(ErrorKind::InvalidEndpoint(e)))
				}),
				integer(),
				decimal()
			))
			.padded_by(whitespace());

        let array_inline = ty
            .clone()
            .padded_by(whitespace())
            .separated_by(just(Token::Comma))
            .allow_trailing()
            .collect()
            .delimited_by(
                just(Token::LeftBracket).padded_by(whitespace()),
                just(Token::RightBracket).padded_by(whitespace()),
            )
            .map(|elems: Vec<TypeExpression>| TypeExpression::StructuralList(elems));

        let array_fixed_inline = ty
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
                    Ok(TypeExpression::FixedSizeList(Box::new(t), n))
                } else {
                    Err(ParseError::new(ErrorKind::InvalidArraySize(format!(
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
           Token::Identifier(k) => TypeExpression::Text(k),
           Token::StringLiteral(k) => TypeExpression::Text(unescape_text(&k)),
        }
        .then(just(Token::Placeholder).or_not())
        .then_ignore(just(Token::Colon).padded_by(whitespace()))
        .then(ty.clone())
        .map(|((name, opt), typ)| {
            if opt.is_some() {
                (name, TypeExpression::Union(vec![typ, TypeExpression::Null]))
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
                TypeExpression::StructuralMap(fields)
            });

        // let generic = select! { Token::Identifier(name) => name }
        //     .then(
        //         ty.clone()
        //             .separated_by(just(Token::Comma).padded_by(whitespace()))
        //             .allow_trailing()
        //             .collect()
        //             .padded_by(whitespace())
        //             .delimited_by(
        //                 just(Token::LeftAngle),
        //                 just(Token::RightAngle),
        //             ),
        //     )
        //     .map(|(name, args): (String, Vec<TypeExpression>)| {
        //         match name.as_str() {
        //             "List" if args.len() == 1 => {
        //                 TypeExpression::StructuralList(Box::new(args[0].clone()))
        //             }
        //             "Map" if args.len() == 2 => {
        //                 let mut it = args.into_iter();
        //                 TypeExpression::StructuralMap(
        //                     Box::new(it.next().unwrap()),
        //                     Box::new(it.next().unwrap()),
        //                 )
        //             }
        //             other => TypeExpression::Generic(other.to_owned(), args),
        //         }
        //     });

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
            .map(
                |(params, ret): (
                    Vec<(String, TypeExpression)>,
                    TypeExpression,
                )| {
                    TypeExpression::Function {
                        parameters: params,
                        return_type: Box::new(ret),
                    }
                },
            );

        let reference = just(Token::Ampersand)
            .ignore_then(just(Token::Mutable).or(just(Token::Final)).or_not())
            .then_ignore(whitespace())
            .then(ty.clone())
            .map(|(maybe_mut, inner): (Option<Token>, TypeExpression)| {
                let mutability = match maybe_mut {
                    Some(Token::Mutable) => ReferenceMutability::Mutable,
                    Some(Token::Final) => ReferenceMutability::Final,
                    None => ReferenceMutability::Immutable,
                    _ => unreachable!(),
                };
                match mutability {
                    ReferenceMutability::Mutable => {
                        TypeExpression::RefMut(Box::new(inner))
                    }
                    ReferenceMutability::Immutable => {
                        TypeExpression::Ref(Box::new(inner))
                    }
                    ReferenceMutability::Final => {
                        TypeExpression::RefFinal(Box::new(inner))
                    }
                }
            });

        let base = choice((
            reference.clone(),
            func.clone(),
            literal.clone(),
            array_inline.clone(),
            array_fixed_inline.clone(),
            structural_map.clone(),
            // generic.clone(),
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

        let postfix_array = just(Token::LeftBracket).ignore_then(choice((
            // Slice: []
            just(Token::RightBracket).to(None),
            // Fixed size: [10]
            integer().then_ignore(just(Token::RightBracket)).map(Some),
        )));

        let array_postfix = base
            .then(postfix_array.repeated().collect::<Vec<_>>())
            .try_map(|(mut t, arrs), _| {
                for arr in arrs {
                    t = match arr {
                        None => TypeExpression::SliceList(Box::new(t)),
                        Some(n) => match integer_to_usize(&n) {
                            Some(size) if size > 0 => {
                                TypeExpression::FixedSizeList(
                                    Box::new(t),
                                    size,
                                )
                            }
                            _ => {
                                return Err(ParseError::new(
                                    ErrorKind::InvalidArraySize(format!(
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
            .map(|(first, mut rest): (TypeExpression, Vec<TypeExpression>)| {
                if rest.is_empty() {
                    return first;
                }
                rest.insert(0, first);
                TypeExpression::Intersection(rest)
            });

        intersection
            .clone()
            .then(
                just(Token::Pipe)
                    .padded_by(whitespace())
                    .ignore_then(intersection.clone())
                    .repeated()
                    .collect(),
            )
            .map(|(first, mut rest): (TypeExpression, Vec<TypeExpression>)| {
                if rest.is_empty() {
                    return first;
                }
                rest.insert(0, first);
                TypeExpression::Union(rest)
            })
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
        .map(|((name, generic), expr)| DatexExpression::TypeDeclaration {
            id: None,
            name: name.to_string(),
            value: expr,
            hoisted: false,
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
        .map(|(name, expr)| DatexExpression::TypeDeclaration {
            id: None,
            name: name.to_string(),
            value: expr,
            hoisted: false,
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
        .map(DatexExpression::Type)
}

#[cfg(test)]
mod tests {
    use crate::ast::{error::src::SrcId, parse};

    use super::*;
    use std::{io, str::FromStr};

    fn parse_unwrap(src: &str) -> DatexExpression {
        let src_id = SrcId::test();
        let res = parse(src);
        if let Err(errors) = res {
            errors.iter().for_each(|e| {
                let cache = ariadne::sources(vec![(src_id, src)]);
                e.clone().write(cache, io::stdout());
            });
            panic!("Parsing errors found");
        }
        res.unwrap()
    }
    fn parse_type_unwrap(src: &str) -> TypeExpression {
        let value = parse_unwrap(format!("type T = {}", src).as_str());
        if let DatexExpression::TypeDeclaration { value, .. } = value {
            value
        } else if let DatexExpression::Statements(statements) = &value
            && statements.len() == 1
        {
            match &statements[0].expression {
                DatexExpression::TypeDeclaration { value, .. } => value.clone(),
                _ => panic!(
                    "Expected TypeDeclaration, got {:?}",
                    statements[0].expression
                ),
            }
        } else {
            panic!("Expected TypeDeclaration, got {:?}", value);
        }
    }

    #[test]
    fn literal() {
        let src = "integer/u16";
        let val = parse_type_unwrap(src);
        assert_eq!(val, TypeExpression::Literal("integer/u16".to_owned()));
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
            TypeExpression::StructuralMap(vec![
                (
                    TypeExpression::Literal("name".to_string()),
                    TypeExpression::Union(vec![
                        TypeExpression::Literal("text".to_owned()),
                        TypeExpression::Null
                    ])
                ),
                (
                    TypeExpression::Literal("age".to_string()),
                    TypeExpression::Union(vec![
                        TypeExpression::Literal("integer".to_owned()),
                        TypeExpression::Literal("text".to_owned())
                    ])
                )
            ])
        );

        // TODO: generics
        // let src = r#"
        //     {
        //         name?: text,
        //         friends: List<&text>
        //     };
        // "#;
        // let val = parse_type_unwrap(src);
        // assert_eq!(
        //     val,
        //     TypeExpression::StructuralMap(vec![
        //         (
        //             TypeExpression::Literal("name".to_string()),
        //             TypeExpression::Union(vec![
        //                 TypeExpression::Literal("text".to_owned()),
        //                 TypeExpression::Null
        //             ])
        //         ),
        //         (
        //             TypeExpression::Literal("friends".to_string()),
        //             TypeExpression::StructuralList(Box::new(TypeExpression::Ref(
        //                 Box::new(TypeExpression::Literal("text".to_owned()))
        //             )))
        //         ),
        //     ])
        // );
        //
        // let src = r#"
		// 	{
    	// 		name: text,
		// 		friends: List<&text>
		// 	}
		// "#;
        // let val = parse_type_unwrap(src);
        // assert_eq!(
        //     val,
        //     TypeExpression::StructuralMap(vec![
        //         (
        //             "name".to_string(),
        //             TypeExpression::Literal("text".to_owned())
        //         ),
        //         (
        //             "friends".to_string(),
        //             TypeExpression::StructuralList(Box::new(TypeExpression::Ref(
        //                 Box::new(TypeExpression::Literal("text".to_owned()))
        //             )))
        //         ),
        //     ])
        // );

        let src = r#"
            {
                name: text,
                age: &mut text
            }
        "#;
        let val = parse_type_unwrap(src);
        assert_eq!(
            val,
            TypeExpression::StructuralMap(vec![
                (
                    TypeExpression::Literal("name".to_string()),
                    TypeExpression::Literal("text".to_owned())
                ),
                (
                    TypeExpression::Literal("age".to_string()),
                    TypeExpression::RefMut(Box::new(TypeExpression::Literal(
                        "text".to_owned()
                    )))
                ),
            ])
        );
    }

    #[test]
    fn union_flat() {
        let src = r#""hello world" | 42"#;
        let val = parse_type_unwrap(src);
        assert_eq!(
            val,
            TypeExpression::Union(vec![
                TypeExpression::Text("hello world".to_owned()),
                TypeExpression::Integer(Integer::from(42)),
            ])
        );

        let src = "1 | 2 | 3 | 4";
        let val = parse_type_unwrap(src);
        assert_eq!(
            val,
            TypeExpression::Union(vec![
                TypeExpression::Integer(Integer::from(1)),
                TypeExpression::Integer(Integer::from(2)),
                TypeExpression::Integer(Integer::from(3)),
                TypeExpression::Integer(Integer::from(4)),
            ])
        );

        let src = "@jonas | @bene";
        let val = parse_type_unwrap(src);
        assert_eq!(
            val,
            TypeExpression::Union(vec![
                TypeExpression::Endpoint(Endpoint::from_str("@jonas").unwrap()),
                TypeExpression::Endpoint(Endpoint::from_str("@bene").unwrap()),
            ])
        );
    }

    #[test]
    fn union_nested() {
        let src = "(1 | 2) | 3 | 4";
        let val = parse_type_unwrap(src);
        assert_eq!(
            val,
            TypeExpression::Union(vec![
                TypeExpression::Union(vec![
                    TypeExpression::Integer(Integer::from(1)),
                    TypeExpression::Integer(Integer::from(2)),
                ]),
                TypeExpression::Integer(Integer::from(3)),
                TypeExpression::Integer(Integer::from(4)),
            ])
        );
    }

    #[test]
    fn union_and_intersection() {
        let src = "1 | (2 & 3) | 4";
        let val = parse_type_unwrap(src);
        assert_eq!(
            val,
            TypeExpression::Union(vec![
                TypeExpression::Integer(Integer::from(1)),
                TypeExpression::Intersection(vec![
                    TypeExpression::Integer(Integer::from(2)),
                    TypeExpression::Integer(Integer::from(3)),
                ]),
                TypeExpression::Integer(Integer::from(4)),
            ])
        );

        let src = "(1 | 2) & 3 & 4";
        let val = parse_type_unwrap(src);
        assert_eq!(
            val,
            TypeExpression::Intersection(vec![
                TypeExpression::Union(vec![
                    TypeExpression::Integer(Integer::from(1)),
                    TypeExpression::Integer(Integer::from(2)),
                ]),
                TypeExpression::Integer(Integer::from(3)),
                TypeExpression::Integer(Integer::from(4)),
            ])
        );
    }

    #[test]
    fn array() {
        let src = "[1, 2, 3, 4]";
        let val = parse_type_unwrap(src);
        assert_eq!(
            val,
            TypeExpression::StructuralList(vec![
                TypeExpression::Integer(Integer::from(1)),
                TypeExpression::Integer(Integer::from(2)),
                TypeExpression::Integer(Integer::from(3)),
                TypeExpression::Integer(Integer::from(4)),
            ])
        );

        let src = "[1,2,text]";
        let val = parse_type_unwrap(src);
        assert_eq!(
            val,
            TypeExpression::StructuralList(vec![
                TypeExpression::Integer(Integer::from(1)),
                TypeExpression::Integer(Integer::from(2)),
                TypeExpression::Literal("text".to_owned()),
            ])
        );

        let src = "[integer|text]";
        let val = parse_type_unwrap(src);
        assert_eq!(
            val,
            TypeExpression::StructuralList(vec![TypeExpression::Union(vec![
                TypeExpression::Literal("integer".to_owned()),
                TypeExpression::Literal("text".to_owned()),
            ])])
        );
    }

    #[test]
    fn array_sized_1() {
        let src = "integer[10]";
        let val = parse_type_unwrap(src);
        assert_eq!(
            val,
            TypeExpression::FixedSizeList(
                Box::new(TypeExpression::Literal("integer".to_owned())),
                10
            )
        );

        let src = "(integer | string)[10]";
        let val = parse_type_unwrap(src);
        assert_eq!(
            val,
            TypeExpression::FixedSizeList(
                Box::new(TypeExpression::Union(vec![
                    TypeExpression::Literal("integer".to_owned()),
                    TypeExpression::Literal("string".to_owned()),
                ])),
                10
            )
        );
    }

    #[test]
    fn array_sized_2() {
        let src = "[text; 4]";
        let val = parse_type_unwrap(src);
        assert_eq!(
            val,
            TypeExpression::FixedSizeList(
                Box::new(TypeExpression::Literal("text".to_owned())),
                4
            )
        );

        let src = "[text;  42]";
        let val = parse_type_unwrap(src);
        assert_eq!(
            val,
            TypeExpression::FixedSizeList(
                Box::new(TypeExpression::Literal("text".to_owned())),
                42
            )
        );

        let src = "[text;10]";
        let val = parse_type_unwrap(src);
        assert_eq!(
            val,
            TypeExpression::FixedSizeList(
                Box::new(TypeExpression::Literal("text".to_owned())),
                10
            )
        );
    }

    #[test]
    fn array_slice() {
        let src = "text[]";
        let val = parse_type_unwrap(src);
        assert_eq!(
            val,
            TypeExpression::SliceList(Box::new(TypeExpression::Literal(
                "text".to_owned()
            )))
        );

        let src = "integer[][][]";
        let val = parse_type_unwrap(src);
        assert_eq!(
            val,
            TypeExpression::SliceList(Box::new(TypeExpression::SliceList(
                Box::new(TypeExpression::SliceList(Box::new(
                    TypeExpression::Literal("integer".to_owned())
                )))
            )))
        );
    }

    // TODO: generics
    // #[test]
    // fn list() {
    //     let src = "List<integer>";
    //     let val = parse_type_unwrap(src);
    //     assert_eq!(
    //         val,
    //         TypeExpression::StructuralList(Box::new(TypeExpression::Literal(
    //             "integer".to_owned()
    //         )))
    //     );
    //
    //     let src = "List<integer | text>";
    //     let val = parse_type_unwrap(src);
    //     assert_eq!(
    //         val,
    //         TypeExpression::StructuralList(Box::new(TypeExpression::Union(vec![
    //             TypeExpression::Literal("integer".to_owned()),
    //             TypeExpression::Literal("text".to_owned()),
    //         ])))
    //     );
    // }

    // #[test]
    // fn map() {
    //     let src = "Map<text, integer>";
    //     let val = parse_type_unwrap(src);
    //     assert_eq!(
    //         val,
    //         TypeExpression::StructuralMap(
    //             Box::new(TypeExpression::Literal("text".to_owned())),
    //             Box::new(TypeExpression::Literal("integer".to_owned()))
    //         )
    //     );
    // }

    #[test]
    fn generic_type() {
        let src = "User<text, integer>";
        let val = parse_type_unwrap(src);
        assert_eq!(
            val,
            TypeExpression::Generic(
                "User".to_owned(),
                vec![
                    TypeExpression::Literal("text".to_owned()),
                    TypeExpression::Literal("integer".to_owned()),
                ],
            )
        );

        let src = "User<text | integer>";
        let val = parse_type_unwrap(src);
        assert_eq!(
            val,
            TypeExpression::Generic(
                "User".to_owned(),
                vec![TypeExpression::Union(vec![
                    TypeExpression::Literal("text".to_owned()),
                    TypeExpression::Literal("integer".to_owned()),
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
            TypeExpression::Function {
                parameters: vec![
                    (
                        "x".to_string(),
                        TypeExpression::Literal("text".to_owned())
                    ),
                    (
                        "y".to_string(),
                        TypeExpression::Union(vec![
                            TypeExpression::Literal("text".to_owned()),
                            TypeExpression::Decimal(
                                Decimal::from_string("4.5").unwrap()
                            )
                        ])
                    )
                ],
                return_type: Box::new(TypeExpression::Union(vec![
                    TypeExpression::Literal("text".to_owned()),
                    TypeExpression::Integer(Integer::from(52))
                ])),
            }
        );

        let src = "(x: &mut text, y: text | 4.5) -> text | 52";
        let val = parse_type_unwrap(src);
        assert_eq!(
            val,
            TypeExpression::Function {
                parameters: vec![
                    (
                        "x".to_string(),
                        TypeExpression::RefMut(Box::new(
                            TypeExpression::Literal("text".to_owned())
                        ))
                    ),
                    (
                        "y".to_string(),
                        TypeExpression::Union(vec![
                            TypeExpression::Literal("text".to_owned()),
                            TypeExpression::Decimal(
                                Decimal::from_string("4.5").unwrap()
                            )
                        ])
                    )
                ],
                return_type: Box::new(TypeExpression::Union(vec![
                    TypeExpression::Literal("text".to_owned()),
                    TypeExpression::Integer(Integer::from(52))
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
            TypeExpression::Ref(Box::new(TypeExpression::StructuralList(vec![
                TypeExpression::RefMut(Box::new(TypeExpression::Literal(
                    "text".to_owned()
                ))),
                TypeExpression::RefMut(Box::new(TypeExpression::Literal(
                    "integer/u8".to_owned()
                ))),
            ])))
        );
    }
}
