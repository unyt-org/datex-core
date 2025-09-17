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
    values::{
        core_values::{
            decimal::{decimal::Decimal, typed_decimal::TypedDecimal},
            endpoint::Endpoint,
            integer::{integer::Integer, typed_integer::TypedInteger},
        },
        reference::ReferenceMutability,
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
                    // match sub.as_deref() {
                    //     None => match base.as_str() {
                    //         "integer" => Some(TypeContainer::integer()),
                    //         "text" => Some(TypeContainer::text()),
                    //         "boolean" => Some(TypeContainer::boolean()),
                    //         "null" => Some(TypeContainer::null()),
                    //         _ => None,
                    //     },
                    //     Some(variant) => match base.as_str() {
                    //         "integer" => IntegerTypeVariant::from_str(variant)
                    //             .ok()
                    //             .map(TypeContainer::typed_integer),
                    //         "decimal" => DecimalTypeVariant::from_str(variant)
                    //             .ok()
                    //             .map(TypeContainer::typed_decimal),
                    //         _ => None,
                    //     },
                    // }
                }),
            // .try_map(|res, _| {
            //     res.ok_or_else(|| ParseError::new(ErrorKind::UnexpectedEnd))
            // }),
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
            .map(|elems: Vec<TypeExpression>| TypeExpression::Array(elems));

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
        let struct_field = select! { Token::Identifier(k) => k }
            .then(just(Token::Placeholder).or_not())
            .then_ignore(just(Token::Colon).padded_by(whitespace()))
            .then(ty.clone())
            .map(|((name, opt), typ)| {
                if opt.is_some() {
                    (
                        name,
                        TypeExpression::Union(vec![typ, TypeExpression::Null]),
                    )
                } else {
                    (name, typ)
                }
            });

        let r#struct = struct_field
            .separated_by(just(Token::Comma).padded_by(whitespace()))
            .allow_trailing()
            .collect()
            .delimited_by(
                just(Token::LeftCurly).padded_by(whitespace()),
                just(Token::RightCurly).padded_by(whitespace()),
            )
            .map(|fields: Vec<(String, TypeExpression)>| {
                TypeExpression::Struct(fields)
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
                match name.as_str() {
                    "List" if args.len() == 1 => {
                        TypeExpression::List(Box::new(args[0].clone()))
                    }
                    "Map" if args.len() == 2 => {
                        let mut it = args.into_iter();
                        TypeExpression::Map(
                            Box::new(it.next().unwrap()),
                            Box::new(it.next().unwrap()),
                        )
                    }
                    other => panic!(
                        "unknown generic type {} with {} arguments",
                        other,
                        args.len()
                    ),
                }
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
            .ignore_then(just(Token::Mutable).or_not())
            .then_ignore(whitespace())
            .then(ty.clone())
            .map(|(maybe_mut, inner): (Option<Token>, TypeExpression)| {
                let mutability = match maybe_mut {
                    Some(_) => ReferenceMutability::Mutable,
                    None => ReferenceMutability::Immutable,
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
            r#struct.clone(),
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
        let optional_postfix_array = base
            .then(
                just(Token::LeftBracket)
                    .ignore_then(just(Token::RightBracket))
                    .repeated()
                    .count(),
            )
            .map(|(base_tc, count): (TypeExpression, usize)| {
                let mut t = base_tc;
                for _ in 0..count {
                    t = TypeExpression::Array(vec![t]);
                }
                t
            });
        // let postfix = base.clone().then(
        // 	choice((
        // 		just(Token::Dot)
        // 			.ignore_then(select! { Token::Identifier(name) => name })
        // 			.map(|name| PostfixOp::Field(name)),

        // 		just(Token::LeftBracket)
        // 			.ignore_then(ty.clone())
        // 			.then_ignore(just(Token::RightBracket))
        // 			.map(|idx| PostfixOp::Index(idx)),
        // 	))
        // 	.repeated()
        // 	.collect(),
        // ).map(|(root, ops): (TypeContainer, Vec<PostfixOp>)| {
        // 	ops.into_iter().fold(root, |acc, op| {
        // 		match op {
        // 			PostfixOp::Field(name) => Type::field_access(acc, name).as_type_container(),
        // 			PostfixOp::Index(idx)  => Type::index_access(acc, idx).as_type_container(),
        // 		}
        // 	})
        // });

        let intersection = optional_postfix_array
            .clone()
            .then(
                // parse zero-or-more `& <postfix_array>`
                just(Token::Ampersand)
                    .padded_by(whitespace())
                    .ignore_then(optional_postfix_array.clone())
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
        // .map(|(first, rest): (TypeContainer, Vec<TypeContainer>)| {
        //     // fold the tail into a single intersection-type
        //     rest.into_iter().fold(first, |acc, next| {
        //         Type::intersection(vec![acc, next]).as_type_container()
        //     })
        // });

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
        // .map(|(first, rest): (TypeContainer, Vec<TypeContainer>)| {
        //     rest.into_iter().fold(first, |acc, next| {
        //         Type::union(vec![acc, next]).as_type_container()
        //     })
        // })
    })
    //.try_map(|res, _| Ok(DatexExpression::Type(res)))
}

pub fn nominal_type_declaration<'a>() -> impl DatexParserTrait<'a> {
    let generic = just(Token::LeftAngle)
        .ignore_then(literal())
        .then_ignore(just(Token::RightAngle))
        .or_not();
    // allow ; and end

    just(Token::Identifier("type".to_string()))
        .padded_by(whitespace())
        .ignore_then(select! { Token::Identifier(name) => name })
        .then(generic)
        .then_ignore(just(Token::Assign).padded_by(whitespace()))
        .then(r#type())
        .padded_by(whitespace())
        .map(|((name, generic), expr)| DatexExpression::TypeDeclaration {
            id: None,
            name: name.to_string(),
            value: expr,
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
    fn r#struct() {
        let src = r#"
			{
				name: text | null,
				age: integer | text
			}
		"#;
        let val = parse_type_unwrap(src);
        assert_eq!(
            val,
            TypeExpression::Struct(vec![
                (
                    "name".to_string(),
                    TypeExpression::Union(vec![
                        TypeExpression::Literal("text".to_owned()),
                        TypeExpression::Null
                    ])
                ),
                (
                    "age".to_string(),
                    TypeExpression::Union(vec![
                        TypeExpression::Literal("integer".to_owned()),
                        TypeExpression::Literal("text".to_owned())
                    ])
                )
            ])
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
            TypeExpression::Struct(vec![
                (
                    "name".to_string(),
                    TypeExpression::Union(vec![
                        TypeExpression::Literal("text".to_owned()),
                        TypeExpression::Null
                    ])
                ),
                (
                    "friends".to_string(),
                    TypeExpression::List(Box::new(TypeExpression::Ref(
                        Box::new(TypeExpression::Literal("text".to_owned()))
                    )))
                ),
            ])
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
            TypeExpression::Struct(vec![
                (
                    "name".to_string(),
                    TypeExpression::Literal("text".to_owned())
                ),
                (
                    "friends".to_string(),
                    TypeExpression::List(Box::new(TypeExpression::Ref(
                        Box::new(TypeExpression::Literal("text".to_owned()))
                    )))
                ),
            ])
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
            TypeExpression::Struct(vec![
                (
                    "name".to_string(),
                    TypeExpression::Literal("text".to_owned())
                ),
                (
                    "age".to_string(),
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
            TypeExpression::Array(vec![
                TypeExpression::Integer(Integer::from(1)),
                TypeExpression::Integer(Integer::from(2)),
                TypeExpression::Integer(Integer::from(3)),
                TypeExpression::Integer(Integer::from(4)),
            ])
        );
    }

    #[test]
    fn array_postfix() {
        let src = "text[]";
        let val = parse_type_unwrap(src);
        assert_eq!(
            val,
            TypeExpression::Array(vec![TypeExpression::Literal(
                "text".to_owned()
            )])
        );

        let src = "integer[][][]";
        let val = parse_type_unwrap(src);
        assert_eq!(
            val,
            TypeExpression::Array(vec![TypeExpression::Array(vec!(
                TypeExpression::Array(vec!(TypeExpression::Literal(
                    "integer".to_owned()
                )))
            ))])
        );

        let src = "[1,2,text]";
        let val = parse_type_unwrap(src);
        assert_eq!(
            val,
            TypeExpression::Array(vec![
                TypeExpression::Integer(Integer::from(1)),
                TypeExpression::Integer(Integer::from(2)),
                TypeExpression::Literal("text".to_owned()),
            ])
        );

        let src = "[integer|text]";
        let val = parse_type_unwrap(src);
        assert_eq!(
            val,
            TypeExpression::Array(vec![TypeExpression::Union(vec![
                TypeExpression::Literal("integer".to_owned()),
                TypeExpression::Literal("text".to_owned()),
            ])])
        );
    }

    #[test]
    fn list() {
        let src = "List<integer>";
        let val = parse_type_unwrap(src);
        assert_eq!(
            val,
            TypeExpression::List(Box::new(TypeExpression::Literal(
                "integer".to_owned()
            )))
        );

        let src = "List<integer | text>";
        let val = parse_type_unwrap(src);
        assert_eq!(
            val,
            TypeExpression::List(Box::new(TypeExpression::Union(vec![
                TypeExpression::Literal("integer".to_owned()),
                TypeExpression::Literal("text".to_owned()),
            ])))
        );
    }

    #[test]
    fn map() {
        let src = "Map<text, integer>";
        let val = parse_type_unwrap(src);
        assert_eq!(
            val,
            TypeExpression::Map(
                Box::new(TypeExpression::Literal("text".to_owned())),
                Box::new(TypeExpression::Literal("integer".to_owned()))
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
            TypeExpression::Ref(Box::new(TypeExpression::Array(vec![
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
