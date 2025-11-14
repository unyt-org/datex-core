use chumsky::{IterParser, Parser, prelude::just, select};

use crate::ast::{
    DatexParserTrait,
    grammar::{
        function::{body, parameter, return_type},
        utils::whitespace,
    },
    lexer::Token,
    spanned::Spanned,
    structs::{
        expression::{
            DatexExpressionData, FunctionDeclaration, InterfaceDeclaration,
        },
        r#type::{TypeExpression, TypeExpressionData},
    },
};

pub fn self_parameter<'a>()
-> impl DatexParserTrait<'a, (String, TypeExpression)> {
    let amp = just(Token::Ampersand).or_not().padded_by(whitespace());
    let mut_tok = just(Token::Mutable).or_not().padded_by(whitespace());

    amp.then(mut_tok)
        .then_ignore(just(Token::Identifier("self".to_string())))
        .map_with(|(amp_opt, mut_opt), span| {
            let ty = match (amp_opt, mut_opt) {
                (None, _) => TypeExpressionData::SelfType, // self
                (Some(_), None) => TypeExpressionData::ReferenceSelf, // &self
                (Some(_), Some(_)) => TypeExpressionData::ReferenceSelfMut, // &mut self
            };
            ("self".to_string(), ty.with_span(span.span()))
        })
        .padded_by(whitespace())
}

pub fn parameters<'a>()
-> impl DatexParserTrait<'a, Vec<(String, TypeExpression)>> {
    let maybe_self = self_parameter().padded_by(whitespace()).or_not();

    let normal_params = parameter()
        .padded_by(whitespace())
        .separated_by(just(Token::Comma).padded_by(whitespace()))
        .collect::<Vec<(String, TypeExpression)>>();

    maybe_self
        .then(
            just(Token::Comma)
                .padded_by(whitespace())
                .ignore_then(normal_params.clone())
                .or(normal_params),
        )
        .map(
            |(self_opt, params): (
                Option<(String, TypeExpression)>,
                Vec<(String, TypeExpression)>,
            )| {
                if let Some(self_param) = self_opt {
                    // Insert self parameter at the front
                    let mut all = Vec::with_capacity(params.len() + 1);
                    all.push(self_param);
                    all.extend(params);
                    all
                } else {
                    params
                }
            },
        )
        .delimited_by(just(Token::LeftParen), just(Token::RightParen))
        .padded_by(whitespace())
}

fn method<'a>(
    statements: impl DatexParserTrait<'a>,
) -> impl DatexParserTrait<'a> {
    let maybe_body = body(statements).map(Some);

    select! { Token::Identifier(name) => name }
        .padded_by(whitespace())
        .then(parameters())
        .then(return_type())
        .then(
            maybe_body
                .or(just(Token::Semicolon).padded_by(whitespace()).to(None))
                .or_not(),
        )
        .map_with(|(((name, params), return_type), body), e| {
            let body_or_noop = body.flatten().unwrap_or_else(|| {
                DatexExpressionData::Noop.with_default_span()
            });
            DatexExpressionData::FunctionDeclaration(FunctionDeclaration {
                name,
                parameters: params,
                return_type,
                body: Box::new(body_or_noop),
            })
            .with_span(e.span())
        })
}

pub fn interface_declaration<'a>(
    statements: impl DatexParserTrait<'a>,
) -> impl DatexParserTrait<'a> {
    let method_decl = method(statements);

    let method_list = method_decl
        .padded_by(whitespace())
        .separated_by(whitespace())
        .collect::<Vec<_>>()
        .padded_by(whitespace())
        .delimited_by(
            just(Token::LeftCurly).padded_by(whitespace()),
            just(Token::RightCurly).padded_by(whitespace()),
        )
        .padded_by(whitespace());

    just(Token::Identifier("interface".to_string()))
        .padded_by(whitespace())
        .ignore_then(select! { Token::Identifier(name) => name })
        .padded_by(whitespace())
        .then(method_list)
        .map_with(|(name, methods), e| {
            DatexExpressionData::InterfaceDeclaration(InterfaceDeclaration {
                name,
                methods,
            })
            .with_span(e.span())
        })
}
