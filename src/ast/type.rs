use std::str::FromStr;

use chumsky::{
    IterParser, Parser,
    prelude::{choice, just, recursive},
    select,
};

use crate::{
    ast::{
        DatexExpression, DatexParserTrait, ParserRecoverExt,
        error::{
            error::{ErrorKind, ParseError},
            pattern::Pattern,
        },
        lexer::{DecimalLiteral, IntegerLiteral, Token, TypedLiteral},
        literal::literal,
        text::unescape_text,
        utils::whitespace,
    },
    values::{
        core_values::{
            decimal::{
                decimal::Decimal,
                typed_decimal::{DecimalTypeVariant, TypedDecimal},
            },
            integer::{
                integer::Integer,
                typed_integer::{IntegerTypeVariant, TypedInteger},
            },
            r#type::{
                Type, structural_type_definition::StructuralTypeDefinition,
            },
        },
        reference::{Reference, ReferenceMutability},
        type_container::TypeContainer,
    },
};

pub fn integer<'a>() -> impl DatexParserTrait<'a, StructuralTypeDefinition> {
    select! {
        Token::DecimalIntegerLiteral(IntegerLiteral { value, variant }) => {
            match variant {
                Some(var) => TypedInteger::from_string_with_variant(&value, var)
                    .map(StructuralTypeDefinition::TypedInteger),
                None => Integer::from_string(&value)
                    .map(StructuralTypeDefinition::Integer),
            }
        },
        Token::BinaryIntegerLiteral(IntegerLiteral { value, variant }) => {
            match variant {
                Some(var) => TypedInteger::from_string_radix_with_variant(&value[2..], 2, var)
                    .map(StructuralTypeDefinition::TypedInteger),
                None => Integer::from_string_radix(&value[2..], 2)
                    .map(StructuralTypeDefinition::Integer),
            }
        },
        Token::HexadecimalIntegerLiteral(IntegerLiteral { value, variant }) => {
            match variant {
                Some(var) => TypedInteger::from_string_radix_with_variant(&value[2..], 16, var)
                    .map(StructuralTypeDefinition::TypedInteger),
                None => Integer::from_string_radix(&value[2..], 16)
                    .map(StructuralTypeDefinition::Integer),
            }
        },
        Token::OctalIntegerLiteral(IntegerLiteral { value, variant }) => {
            match variant {
                Some(var) => TypedInteger::from_string_radix_with_variant(&value[2..], 8, var)
                    .map(StructuralTypeDefinition::TypedInteger),
                None => Integer::from_string_radix(&value[2..], 8)
                    .map(StructuralTypeDefinition::Integer),
            }
        },
    }.try_map(|res, _| {
		res.map_err(|e| ParseError::new(ErrorKind::NumberParseError(e)))
	})
}

pub fn decimal<'a>() -> impl DatexParserTrait<'a, StructuralTypeDefinition> {
    select! {
        Token::DecimalLiteral(DecimalLiteral { value, variant }) => {
            match variant {
                Some(var) => TypedDecimal::from_string_and_variant_in_range(&value, var).map(StructuralTypeDefinition::TypedDecimal),
                None => Decimal::from_string(&value).map(StructuralTypeDefinition::Decimal)
            }
        },
        Token::FractionLiteral(s) => Decimal::from_string(&s).map(StructuralTypeDefinition::Decimal),
    }.try_map(|res, _| {
		res.map_err(|e| ParseError::new(ErrorKind::NumberParseError(e)))
	})
}

pub fn r#type<'a>() -> impl DatexParserTrait<'a, TypeContainer> {
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
                        None => match base.as_str() {
                            "integer" => Some(TypeContainer::integer()),
                            "text" => Some(TypeContainer::text()),
                            "boolean" => Some(TypeContainer::boolean()),
                            "null" => Some(TypeContainer::null()),
                            _ => None,
                        },
                        Some(variant) => match base.as_str() {
                            "integer" => IntegerTypeVariant::from_str(variant)
                                .ok()
                                .map(TypeContainer::typed_integer),
                            "decimal" => DecimalTypeVariant::from_str(variant)
                                .ok()
                                .map(TypeContainer::typed_decimal),
                            _ => None,
                        },
                    }
                })
                .try_map(|res, _| {
                    res.ok_or_else(|| ParseError::new(ErrorKind::UnexpectedEnd))
                }),
            just(Token::Null).map(|_| TypeContainer::null()),
        ));

        let literal =
            choice((
				select! {
					Token::StringLiteral(s) => StructuralTypeDefinition::Text(unescape_text(&s).into()),
				},
				select! {
					Token::True => StructuralTypeDefinition::Boolean(true.into()),
					Token::False => StructuralTypeDefinition::Boolean(false.into()),
				},
				integer(),
				decimal()
			))
			.padded_by(whitespace())
			.map(|value: StructuralTypeDefinition| {
                Type::structural(value)
                    .as_type_container()
            });

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
            .map(|elems: Vec<TypeContainer>| {
                Type::array(elems).as_type_container()
            });

        let key_ident =
            select! { Token::Identifier(k) => k }.padded_by(whitespace());
        let r#struct = key_ident
            .clone()
            .then_ignore(just(Token::Colon))
            .padded_by(whitespace())
            .then(ty.clone())
            .padded_by(whitespace())
            .separated_by(just(Token::Comma))
            .allow_trailing()
            .collect()
            .delimited_by(
                just(Token::LeftCurly).padded_by(whitespace()),
                just(Token::RightCurly).padded_by(whitespace()),
            )
            .map(|fields: Vec<(String, TypeContainer)>| {
                Type::r#struct(fields).as_type_container()
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
            .map(|(name, args): (String, Vec<TypeContainer>)| {
                match name.as_str() {
                    "List" if args.len() == 1 => {
                        Type::list(args.into_iter().next().unwrap())
                            .as_type_container()
                    }
                    "Map" if args.len() == 2 => {
                        let mut it = args.into_iter();
                        Type::map(it.next().unwrap(), it.next().unwrap())
                            .as_type_container()
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
                    Vec<(String, TypeContainer)>,
                    TypeContainer,
                )| {
                    Type::function(params, ret).as_type_container()
                },
            );

        let reference = just(Token::Ampersand)
            .ignore_then(just(Token::Mutable).or_not())
            .then_ignore(whitespace())
            .then(ty.clone())
            .map(|(maybe_mut, inner): (Option<Token>, TypeContainer)| {
                let mutability = match maybe_mut {
                    Some(_) => ReferenceMutability::Mutable,
                    None => ReferenceMutability::Immutable,
                };
                let t = match inner {
                    TypeContainer::Type(mut ty) => {
                        ty.reference_mutability = Some(mutability);
                        ty
                    }
                    TypeContainer::TypeReference(r) => Type::reference(
                        Reference::TypeReference(r),
                        Some(mutability),
                    ),
                };

                TypeContainer::Type(t)
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

        // parse zero-or-more postfix `[]`
        let optional_postfix_array = base
            .then(
                just(Token::LeftBracket)
                    .ignore_then(just(Token::RightBracket))
                    .repeated()
                    .count(),
            )
            .map(|(base_tc, count): (TypeContainer, usize)| {
                let mut t = base_tc;
                for _ in 0..count {
                    t = Type::array(vec![t]).as_type_container();
                }
                t
            });

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
            .map(|(first, rest): (TypeContainer, Vec<TypeContainer>)| {
                // fold the tail into a single intersection-type
                rest.into_iter().fold(first, |acc, next| {
                    Type::intersection(vec![acc, next]).as_type_container()
                })
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
            .map(|(first, rest): (TypeContainer, Vec<TypeContainer>)| {
                rest.into_iter().fold(first, |acc, next| {
                    Type::union(vec![acc, next]).as_type_container()
                })
            })
    })
}

pub fn type_declaration<'a>() -> impl DatexParserTrait<'a> {
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
        .then_ignore(just(Token::Semicolon).or_not().padded_by(whitespace()))
        .map(|((name, generic), expr)| DatexExpression::TypeDeclaration {
            id: None,
            name: name.to_string(),
            value: Box::new(expr),
        })
        .labelled(Pattern::Declaration)
        .as_context()
}
