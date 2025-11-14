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
    let amp = just(Token::Ampersand);
    let mut_tok = just(Token::Mutable);

    // &mut self | &self
    let ref_forms = amp
        .ignore_then(mut_tok.clone().or_not().padded_by(whitespace()))
        .ignore_then(just(Token::Identifier("self".into())))
        .to(("self".to_string(), TypeExpressionData::ReferenceSelf))
        .map_with(|(name, data), e| (name, data.with_span(e.span())));

    // self | mut self
    let plain_forms = mut_tok
        .or_not()
        .ignore_then(just(Token::Identifier("self".into())))
        .to(("self".to_string(), TypeExpressionData::ReferenceSelf))
        .map_with(|(name, data), e| (name, data.with_span(e.span())));

    ref_forms.or(plain_forms).padded_by(whitespace())
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
