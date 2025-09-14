use chumsky::{
    IterParser, Parser,
    prelude::{choice, just, recursive},
    select,
};

use crate::{
    ast::{
        DatexExpression, DatexParserTrait,
        error::pattern::Pattern,
        lexer::{Token, TypedLiteral},
        literal::literal,
        utils::whitespace,
    },
    values::{
        core_values::{
            decimal::{decimal::Decimal, typed_decimal::TypedDecimal},
            r#type::{
                Type, structural_type_definition::StructuralTypeDefinition,
            },
        },
        type_container::TypeContainer,
    },
};

// pub fn r#type<'a>() -> impl DatexParserTrait<'a, TypeContainer> {
//     recursive(|ty| {
//         // primitives
//         let primitive = select! {
//             Token::Identifier(s) => s
//         }
//         .map(|s: String| match s.as_str() {
//             "integer" => TypeContainer::integer(),
//             "text" => TypeContainer::text(),
//             "boolean" => TypeContainer::boolean(),
//             _ => panic!("unknown primitive type {}", s),
//         });

//         let literal = select! {
//             Token::DecimalLiteral(n) => Type::from(Decimal::from_string(&n.value).unwrap().into()).as_type_container(),
//         };

//         // arrays: [t1, t2, ...]
//         let array_inline: _ = ty
//             .clone()
//             .separated_by(just(Token::Comma))
//             .allow_trailing()
//             .delimited_by(just(Token::LeftBracket), just(Token::RightBracket))
//             .map(|elems: Vec<TypeContainer>| {
//                 Type::array(elems).as_type_container()
//             });

//         let key_ident: _ = select! { Token::Identifier(k) => k };
//         // The inner parser yields (String, TypeContainer), separated_by gives Vec<(String, TypeContainer)>
//         let strukt: _ = key_ident
//             .then_ignore(just(Token::Colon))
//             .then(ty.clone())
//             .separated_by(just(Token::Comma))
//             .allow_trailing()
//             .delimited_by(just(Token::LeftCurly), just(Token::RightCurly))
//             .map(|fields: Vec<(String, TypeContainer)>| {
//                 Type::structural(StructuralTypeDefinition::Struct(fields))
//                     .as_type_container()
//             });

//         // generics: List<T>, Map<K,V>
//         let generic: _ = select! { Token::Identifier(name) => name }
//             .then(
//                 ty.clone()
//                     .separated_by(just(Token::Comma))
//                     .allow_trailing()
//                     .delimited_by(
//                         just(Token::LeftAngle),
//                         just(Token::RightAngle),
//                     ),
//             )
//             .map(|(name, args): (String, Vec<TypeContainer>)| {
//                 match name.as_str() {
//                     "List" if args.len() == 1 => {
//                         Type::structural(StructuralTypeDefinition::List(
//                             Box::new(args[0].clone()),
//                         ))
//                         .as_type_container()
//                     } // << CHANGED: returns TypeContainer
//                     "Map" if args.len() == 2 => {
//                         let mut it = args.into_iter();
//                         let k = it.next().unwrap();
//                         let v = it.next().unwrap();
//                         Type::structural(StructuralTypeDefinition::Map(
//                             Box::new((k, v)),
//                         ))
//                         .as_type_container()
//                     }
//                     // Fallback: unknown generic — treat as ident (or extend StructuralTypeDefinition)
//                     other => panic!(
//                         "unknown generic type {} with {} arguments",
//                         other,
//                         args.len()
//                     ),
//                 }
//             });

//         let base: _ = choice((
//             primitive.clone(),
//             literal.clone(),
//             // array_inline.clone(),
//             // strukt.clone(),
//             // generic.clone(),
//         ));

//         let postfix_array: _ = base
//             .then(
//                 just(Token::LeftBracket)
//                     .ignore_then(just(Token::RightBracket))
//                     .repeated(), // returns Vec<()>
//             )
//             .map(|(base_tc, arrs): (TypeContainer, Vec<()>)| {
//                 let mut t = base_tc;
//                 for _ in arrs {
//                     t = Type::structural(StructuralTypeDefinition::List(
//                         Box::new(t),
//                     ))
//                     .as_type_container();
//                 }
//                 t
//             });

//         choice((
//             postfix_array,
//             generic,
//             primitive,
//             literal,
//             array_inline,
//             strukt,
//         ))
//     })
// }

pub fn r#type<'a>() -> impl DatexParserTrait<'a, TypeContainer> {
    recursive(|ty| {
        let primitive =
            select! { Token::Identifier(s) => s }.map(|s: String| {
                match s.as_str() {
                    "integer" => TypeContainer::integer(),
                    "text" => TypeContainer::text(),
                    "boolean" => TypeContainer::boolean(),
                    _ => panic!("unknown primitive type {}", s),
                }
            });

        let literal =
            select! { Token::DecimalLiteral(TypedLiteral { value, .. }) => value }
            .padded_by(whitespace())
			.map(|value: String| {
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
            .then_ignore(just(Token::Colon))
            .padded_by(whitespace())
            .then(ty.clone())
            .separated_by(just(Token::Comma))
            .allow_trailing() // << CHANGED
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
                    .separated_by(just(Token::Comma))
                    .allow_trailing() // << CHANGED: allow trailing commas in generics too
                    .collect()
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
        let base = choice((
            primitive.clone(),
            literal.clone(),
            array_inline.clone(),
            r#struct.clone(),
            generic.clone(),
        ));

        let postfix_array = base
            .then(
                just(Token::LeftBracket)
                    .ignore_then(just(Token::RightBracket))
                    .repeated()
                    .at_least(1)
                    .count(),
            )
            .map(|(base_tc, count): (TypeContainer, usize)| {
                let mut t = base_tc;
                for _ in 0..count {
                    t = Type::array(vec![t]).as_type_container();
                }
                t
            });

        choice((
            postfix_array,
            generic,
            primitive,
            literal,
            array_inline,
            r#struct,
        ))
    })
}

pub fn type_declaration<'a>() -> impl DatexParserTrait<'a> {
    let generic = just(Token::LeftAngle)
        .ignore_then(literal())
        .then_ignore(just(Token::RightAngle))
        .or_not();

    just(Token::Identifier("type".to_string()))
        .padded_by(whitespace())
        .ignore_then(select! { Token::Identifier(name) => name })
        .then(generic)
        .then_ignore(just(Token::Assign).padded_by(whitespace()))
        .then(r#type())
        .map(|((name, generic), expr)| DatexExpression::TypeDeclaration {
            id: None,
            name: name.to_string(),
            value: Box::new(expr),
        })
        .labelled(Pattern::Declaration)
        .as_context()
}
