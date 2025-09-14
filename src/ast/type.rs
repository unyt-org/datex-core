use chumsky::{
    IterParser, Parser,
    prelude::{choice, just, recursive},
    select,
};

use crate::{
    ast::{
        error::pattern::Pattern, lexer::{IntegerLiteral, Token, TypedLiteral}, literal::literal, utils::whitespace, DatexExpression, DatexParserTrait
    },
    values::{
        core_values::{
            decimal::{decimal::Decimal, typed_decimal::TypedDecimal}, integer::integer::Integer, r#type::{
                structural_type_definition::StructuralTypeDefinition, Type
            }
        },
        type_container::TypeContainer,
    },
};

pub fn r#type<'a>() -> impl DatexParserTrait<'a, TypeContainer> {
    recursive(|ty| {

		let paren_group = ty
			.clone()
			.delimited_by(
				just(Token::LeftParen).padded_by(whitespace()),
				just(Token::RightParen).padded_by(whitespace()),
			);

        // Parse a type reference, e.g. `integer`, `text`, `User` etc.
        let type_reference =
            select! { Token::Identifier(s) => s }.map(|s: String| {
                match s.as_str() {
                    "integer" => TypeContainer::integer(),
                    "text" => TypeContainer::text(),
                    "boolean" => TypeContainer::boolean(),
					"null" => TypeContainer::null(),
                    _ => panic!("unknown primitive type {}", s),
                }
            }).or(just(Token::Null).map(|_| TypeContainer::null()));

        let literal =
            select! { 
				Token::DecimalLiteral(TypedLiteral { value, variant }) => {
					if let Some(variant) = variant {
						StructuralTypeDefinition::TypedDecimal(
							TypedDecimal::from_string_and_variant(&value, variant).unwrap()
						)
					} else {
						StructuralTypeDefinition::Decimal(
							Decimal::from_string(&value).unwrap()
						)
					}
				},
				Token::DecimalIntegerLiteral(IntegerLiteral {value, variant}) => StructuralTypeDefinition::Integer(
					Integer::from_string(&value).unwrap()
				),
				Token::StringLiteral(s) => StructuralTypeDefinition::Text(s.into()),
			}
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
        let r#struct = key_ident.clone()
            .then_ignore(just(Token::Colon))
            .padded_by(whitespace())
            .then(ty.clone())
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
			.map(|(params, ret): (Vec<(String, TypeContainer)>, TypeContainer)| {
				Type::function(params, ret).as_type_container()
			});

        let base = choice((
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
