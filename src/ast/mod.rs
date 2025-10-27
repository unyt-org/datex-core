pub mod assignment_operation;
pub mod atom;
pub mod binary_operation;
pub mod binding;
pub mod chain;
pub mod comparison_operation;
pub mod data;
pub mod decimal;
pub mod endpoint;
pub mod error;
pub mod function;
pub mod integer;
pub mod key;
pub mod lexer;
pub mod list;
pub mod literal;
pub mod map;
pub mod parse_result;
pub mod text;
pub mod r#type;
pub mod unary;
pub mod unary_operation;
pub mod utils;
pub mod visitor;
use crate::ast::atom::*;
use crate::ast::binary_operation::*;
use crate::ast::binding::*;
use crate::ast::chain::*;
use crate::ast::comparison_operation::*;
use crate::ast::data::expression::Conditional;
use crate::ast::data::expression::RemoteExecution;
use crate::ast::data::spanned::Spanned;
use crate::ast::error::error::ParseError;
use crate::ast::error::pattern::Pattern;
use crate::ast::function::*;
use crate::ast::key::*;
use crate::ast::list::*;
use crate::ast::map::*;
use crate::ast::r#type::type_expression;
use crate::ast::unary::*;
use crate::ast::utils::*;

use crate::ast::data::expression::{
    DatexExpression, DatexExpressionData, Statements,
};
use crate::ast::parse_result::{
    DatexParseResult, InvalidDatexParseResult, ValidDatexParseResult,
};
use chumsky::extra::Err;
use chumsky::prelude::*;
use lexer::Token;
use logos::Logos;
use std::ops::Range;

pub type TokenInput<'a, X = Token> = &'a [X];
pub trait DatexParserTrait<'a, T = DatexExpression> =
    Parser<'a, TokenInput<'a>, T, Err<ParseError>> + Clone + 'a;

pub type DatexScriptParser<'a> =
    Boxed<'a, 'a, TokenInput<'a>, DatexExpression, Err<ParseError>>;

pub trait ParserRecoverExt<'a, I>:
    DatexParserTrait<'a, Result<DatexExpression, I>>
where
    I: 'a + Into<ParseError>,
{
    fn recover_invalid(self) -> impl DatexParserTrait<'a, DatexExpression>
    where
        Self: Sized,
    {
        self.validate(
            |item: Result<DatexExpression, I>,
             ctx,
             emitter: &mut chumsky::input::Emitter<ParseError>| {
                match item {
                    Ok(expr) => expr,
                    Err(err) => {
                        let span = ctx.span();
                        let mut error: ParseError = err.into();
                        error.set_token_pos(span.start);
                        emitter.emit(error);
                        DatexExpressionData::Recover.with_span(span)
                    }
                }
            },
        )
    }
}

impl<'a, I, P> ParserRecoverExt<'a, I> for P
where
    I: 'a + Into<ParseError>,
    P: DatexParserTrait<'a, Result<DatexExpression, I>>,
{
}

pub fn create_parser<'a>() -> impl DatexParserTrait<'a, DatexExpression> {
    // an expression
    let mut inner_expression = Recursive::declare();

    // an expression or remote execution
    let mut expression = Recursive::declare();

    // a sequence of expressions, separated by semicolons, optionally terminated with a semicolon
    let statements = expression
        .clone()
        .then_ignore(
            just(Token::Semicolon)
                .padded_by(whitespace())
                .repeated()
                .at_least(1),
        )
        .repeated()
        .collect::<Vec<_>>()
        .then(
            expression
                .clone()
                .then(just(Token::Semicolon).padded_by(whitespace()).or_not())
                .or_not(), // Final expression with optional semicolon
        )
        .map_with(|(statements, last), e| {
            // Convert expressions with mandatory semicolon
            let mut statements: Vec<DatexExpression> = statements;
            let mut is_terminated = true;

            if let Some((last_statement, last_semi)) = last {
                // add last_expr to statements
                statements.push(last_statement);
                is_terminated = last_semi.is_some();
            }
            // if single statement without semicolon, treat it as a single expression
            if statements.len() == 1 && !is_terminated {
                statements.remove(0)
            } else {
                DatexExpressionData::Statements(Statements {
                    statements,
                    is_terminated,
                })
                .with_span(e.span())
            }
        })
        .boxed()
        .labelled(Pattern::Custom("statements"));

    // expression wrapped in parentheses
    let wrapped_expression = statements
        .clone()
        .delimited_by(just(Token::LeftParen), just(Token::RightParen))
        .map_with(|inner, _| {
            let mut expr = inner;
            expr.wrapped = Some(expr.wrapped.unwrap_or(0).saturating_add(1));
            expr
        });

    // a valid map/list key
    // abc, a, "1", "test", (1 + 2), ...
    let key = key(wrapped_expression.clone()).labelled(Pattern::Custom("key"));

    // list
    // 1,2,3
    // [1,2,3,4,13434,(1),4,5,7,8]
    let list = list(expression.clone());

    // map
    let map = map(key.clone(), expression.clone());

    // atomic expression (e.g. 1, "text", (1 + 2), (1;2))
    let atom = atom(list.clone(), map.clone(), wrapped_expression.clone());
    let unary = unary(atom.clone());

    // apply chain: two expressions following each other directly, optionally separated with "." (property access)
    let chain =
        chain(unary.clone(), key.clone(), atom.clone(), expression.clone());

    let binary = binary_operation(chain);

    // FIXME #363 WIP
    let function_declaration = function(statements.clone());

    // comparison (==, !=, is, …)
    let comparison = comparison_operation(binary.clone());

    // declarations or assignments
    let declaration_or_assignment =
        declaration_or_assignment(expression.clone(), unary.clone());

    let condition_union = binary_operation(chain_without_whitespace_apply(
        unary.clone(),
        key.clone(),
        expression.clone(),
    ));
    let condition = comparison_operation(condition_union);

    let if_expression = recursive(|if_rec| {
        just(Token::If)
            .padded_by(whitespace())
            .ignore_then(condition.clone())
            .then(
                choice((
                    wrapped_expression.clone(),
                    list.clone(),
                    map.clone(),
                    statements.clone(),
                    unary.clone(),
                ))
                .padded_by(whitespace()),
            )
            .then(
                just(Token::Else)
                    .padded_by(whitespace())
                    .ignore_then(choice((
                        if_rec.clone(),
                        wrapped_expression.clone(),
                        list.clone(),
                        map.clone(),
                        statements.clone(),
                        unary.clone(),
                    )))
                    .or_not(),
            )
            .map_with(|((cond, then_branch), else_opt), e| {
                DatexExpressionData::Conditional(Conditional {
                    condition: Box::new(cond),
                    then_branch: Box::new(unwrap_single_statement(then_branch)),
                    else_branch: else_opt
                        .map(unwrap_single_statement)
                        .map(Box::new),
                })
                .with_span(e.span())
            })
            .boxed()
    });

    // expression :: expression
    let remote_execution = inner_expression
        .clone()
        .then_ignore(just(Token::DoubleColon).padded_by(whitespace()))
        .then(inner_expression.clone())
        .map_with(|(endpoint, expr), e| {
            DatexExpressionData::RemoteExecution(RemoteExecution {
                left: Box::new(endpoint),
                right: Box::new(expr),
            })
            .with_span(e.span())
        });

    inner_expression.define(
        choice((
            type_expression(),
            if_expression,
            declaration_or_assignment,
            function_declaration,
            comparison,
        ))
        .padded_by(whitespace()),
    );

    expression.define(choice((remote_execution, inner_expression.clone())));

    choice((
        // empty script (0-n semicolons)
        just(Token::Semicolon)
            .repeated()
            .at_least(1)
            .padded_by(whitespace())
            .map_with(|_, e| {
                DatexExpressionData::Statements(Statements::empty())
                    .with_span(e.span())
            }),
        // statements
        statements,
    ))
}

/// Parse the given source code into a DatexExpression AST.
/// Returns either the AST and the spans of each token, or a list of parse errors if parsing failed.
pub fn parse(mut src: &str) -> DatexParseResult {
    // strip shebang at beginning of the source code
    if src.starts_with("#!") {
        if let Some(pos) = src.find('\n') {
            src = &src[pos + 1..];
        } else {
            src = "";
        }
    }

    // lex the source code
    let tokens = Token::lexer(src);
    let tokens_spanned_result: Result<Vec<(Token, Range<usize>)>, _> = tokens
        .spanned()
        .map(|(tok, span)| {
            tok.map(|t| (t, span.clone()))
                .map_err(|_| ParseError::new_unexpected_with_span(None, span))
        })
        .collect::<Result<_, _>>();
    // return early if lexing failed
    if let Err(err) = &tokens_spanned_result {
        return DatexParseResult::Invalid(InvalidDatexParseResult {
            ast: None,
            errors: vec![err.clone()],
            spans: vec![],
        });
    }
    let tokens_spanned = tokens_spanned_result.unwrap();

    let (tokens, spans): (Vec<_>, Vec<_>) = tokens_spanned.into_iter().unzip();
    let parser = create_parser();
    let result = parser.parse(&tokens);
    if !result.has_errors() {
        DatexParseResult::Valid(ValidDatexParseResult {
            ast: result.into_output().unwrap(),
            spans,
        })
    } else {
        DatexParseResult::Invalid(InvalidDatexParseResult {
            errors: result
                .errors()
                .map(|e| {
                    let mut owned_error: ParseError = e.clone();
                    let mut index = owned_error.token_pos().unwrap();
                    if index >= spans.len() {
                        // FIXME #364 how to show file end?
                        index = spans.len() - 1;
                    }
                    let span = spans.get(index).unwrap();
                    owned_error.set_span(span.clone());
                    owned_error
                })
                .collect(),
            ast: result.into_output(),
            spans,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        ast::{
            assignment_operation::AssignmentOperator,
            data::{
                expression::{
                    ApplyChain, BinaryOperation, ComparisonOperation,
                    FunctionDeclaration, TypeDeclaration,
                },
                spanned::Spanned,
                r#type::{
                    Intersection, SliceList, StructuralMap, TypeExpression,
                    TypeExpressionData, Union,
                },
            },
            error::{error::ErrorKind, pattern::Pattern, src::SrcId},
            unary_operation::{
                ArithmeticUnaryOperator, LogicalUnaryOperator, UnaryOperator,
            },
        },
        values::{
            core_values::{
                decimal::Decimal,
                endpoint::{Endpoint, InvalidEndpointError},
                integer::{Integer, typed_integer::TypedInteger},
            },
            pointer::PointerAddress,
            value_container::ValueContainer,
        },
    };

    use super::*;
    use crate::ast::data::expression::{
        DatexExpressionData, List, Map, Slot, UnaryOperation,
        VariableDeclaration, VariableKind,
    };
    use datex_core::ast::data::expression::VariableAssignment;
    use std::{
        assert_matches::assert_matches, collections::HashMap, io, str::FromStr,
        vec,
    };

    /// Parse the given source code into a DatexExpression AST.
    fn parse_unwrap(src: &str) -> DatexExpression {
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
            DatexParseResult::Valid(ValidDatexParseResult { ast, .. }) => ast,
        }
    }

    /// Parse the given source code into a DatexExpressionData AST.
    /// Will panic if there are any parse errors.
    fn parse_unwrap_data(src: &str) -> DatexExpressionData {
        parse_unwrap(src).data
    }

    /// Parse the given source code into a DatexExpression AST.
    /// If there are any parse errors, they will be printed to stdout.
    fn parse_print_error(
        src: &str,
    ) -> Result<DatexExpression, Vec<ParseError>> {
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
                Err(errors)
            }
            DatexParseResult::Valid(ValidDatexParseResult { ast, .. }) => {
                Ok(ast)
            }
        }
    }

    /// Helper function to parse a source string and convert it to a ValueContainer.
    /// Panics if parsing or conversion fails.
    fn parse_to_value_container(src: &str) -> ValueContainer {
        let expr = parse_unwrap_data(src);
        ValueContainer::try_from(&expr).unwrap_or_else(|_| {
            panic!("Failed to convert expression to ValueContainer")
        })
    }

    #[test]
    fn json() {
        let src = r#"
            {
                "name": "Test",
                "value": 42,
                "active": true,
                "items": [1, 2, 3, 0.5],
                "nested": {
                    "key": "value"
                }
            }
        "#;

        let json = parse_unwrap_data(src);

        assert_eq!(
            json,
            DatexExpressionData::Map(Map::new(vec![
                (
                    DatexExpressionData::Text("name".to_string())
                        .with_default_span(),
                    DatexExpressionData::Text("Test".to_string())
                        .with_default_span()
                ),
                (
                    DatexExpressionData::Text("value".to_string())
                        .with_default_span(),
                    DatexExpressionData::Integer(Integer::from(42))
                        .with_default_span()
                ),
                (
                    DatexExpressionData::Text("active".to_string())
                        .with_default_span(),
                    DatexExpressionData::Boolean(true).with_default_span()
                ),
                (
                    DatexExpressionData::Text("items".to_string())
                        .with_default_span(),
                    DatexExpressionData::List(List::new(vec![
                        DatexExpressionData::Integer(Integer::from(1))
                            .with_default_span(),
                        DatexExpressionData::Integer(Integer::from(2))
                            .with_default_span(),
                        DatexExpressionData::Integer(Integer::from(3))
                            .with_default_span(),
                        DatexExpressionData::Decimal(
                            Decimal::from_string("0.5").unwrap()
                        )
                        .with_default_span()
                    ]))
                    .with_default_span()
                ),
                (
                    DatexExpressionData::Text("nested".to_string())
                        .with_default_span(),
                    DatexExpressionData::Map(Map::new(
                        vec![(
                            DatexExpressionData::Text("key".to_string())
                                .with_default_span(),
                            DatexExpressionData::Text("value".to_string())
                                .with_default_span()
                        )]
                        .into_iter()
                        .collect()
                    ))
                    .with_default_span()
                ),
            ]))
        );
    }

    #[test]
    #[ignore = "WIP"]
    fn type_expression() {
        let src = "type(1 | 2)";
        let result = parse_print_error(src);
        let expr = result.unwrap().data;
        assert_matches!(
            expr,
            DatexExpressionData::Type(TypeExpression {
                data: TypeExpressionData::Union(_),
                ..
            })
        );

        let src = "var a = type(1,2,3)";
        let result = parse_print_error(src);
        let expr = result.unwrap().data;
        if let DatexExpressionData::VariableDeclaration(VariableDeclaration {
            init_expression: value,
            ..
        }) = expr
        {
            assert_matches!(
                *value,
                DatexExpression {
                    data: DatexExpressionData::Type(TypeExpression {
                        data: TypeExpressionData::StructuralList(_),
                        ..
                    }),
                    ..
                }
            );
        } else {
            panic!("Expected VariableDeclaration");
        }
    }

    #[test]
    fn structural_type_declaration() {
        let src = "typedef A = integer";
        let result = parse_print_error(src);
        let expr = result.unwrap();
        assert_matches!(expr,
            DatexExpression {
                data: DatexExpressionData::TypeDeclaration(TypeDeclaration { name, .. }), ..
            }
            if name == "A"
        );
    }

    #[test]
    fn nominal_type_declaration() {
        let src = "type B = { x: integer, y: string }";
        let result = parse_print_error(src);
        let expr = result.unwrap();
        assert_matches!(expr,
            DatexExpression {
                data: DatexExpressionData::TypeDeclaration(TypeDeclaration { name, .. }), ..
            }
            if name == "B"
        );

        let src = "type User<T> = {id: T}";
        let result = parse_print_error(src);
        let expr = result.unwrap();
        assert_matches!(expr,
            DatexExpression {
                data: DatexExpressionData::TypeDeclaration(TypeDeclaration { name, .. }), ..
            }
            if name == "User"
        );

        let src = "type User/admin = {id: integer}";
        let result = parse_print_error(src);
        let expr = result.unwrap();
        assert_matches!(expr,
            DatexExpression {
                data: DatexExpressionData::TypeDeclaration(TypeDeclaration { name, .. }), ..
            }
            if name == "User/admin"
        );
    }

    /// # WIP
    /// This test is a WIP item, that should allow us to provide context to the grammar error recovery.
    #[test]
    #[ignore = "WIP"]
    fn test_parse_error_unclosed_delimiter() {
        let src = r#"[1,,]"#;
        let result = parse_print_error(src);

        let src = r#"var x"#;
        let result = parse_print_error(src);

        let src = r#"var x = =1"#;
        let result = parse_print_error(src);

        let src = r#"var x = (1, 2, [10, 20, {1:2})] + 4"#;
        let result = parse_print_error(src);

        let src = r#"[1, )]"#;
        let result = parse_print_error(src);

        let src = r#"(1 + 2 + ])"#;
        let result = parse_print_error(src);

        let src = r#"{x: 1 + +}"#;
        let result = parse_print_error(src);

        let src = r#"(1: x, 2: 1 + +)"#;
        let result = parse_print_error(src);

        // let src = r#"
        // var x = (5 + 3;
        // var y = 42;
        // "#;
        // let result = parse_print_error(src);
        // println!("{:?}", result);
        // let errors = result.err().unwrap();
        // assert_eq!(errors.len(), 3);
        // let error1 = errors[0].clone();
        // assert_matches!(
        //     error1.kind(),
        //     ErrorKind::Unexpected {
        //         found: None,
        //         expected: _,
        //     }
        // );
        // assert_eq!(error1.span(), Some(17..18));
        // let error2 = errors[1].clone();
        // assert_matches!(
        //     error2.kind(),
        //     ErrorKind::Unexpected {
        //         found: None,
        //         expected: _,
        //     }
        // );
        // assert_eq!(error2.span(), Some(45..46));
        // let error3 = errors[2].clone();
        // assert_matches!(
        //     error3.kind(),
        //     ErrorKind::Unexpected {
        //         found: None,
        //         expected: _,
        //     }
        // );
        // assert_eq!(error3.span(), Some(73..74));
    }

    #[test]
    fn parse_error_endpoint() {
        let src = "@j0Onas";
        let result = parse_print_error(src);
        let errors = result.err().unwrap();
        assert_eq!(errors.len(), 1);
        let error = errors[0].clone();
        assert_matches!(
            error.kind(),
            ErrorKind::InvalidEndpoint(InvalidEndpointError::InvalidCharacters)
        );
        assert_eq!(error.span(), Some(0..7));
    }

    #[test]
    fn parse_error_missing_token() {
        let src = r#"
        var x = 52; var y = ; 
        var y = 5
        "#;
        let result = parse_print_error(src);
        let errors = result.err().unwrap();
        assert_eq!(errors.len(), 1);
        let error = errors[0].clone();
        assert_matches!(
            error.kind(),
            ErrorKind::Unexpected {
                found: Some(Pattern::Token(Token::Semicolon)),
                ..
            }
        );
        assert_eq!(error.span(), Some(29..30));
    }

    #[test]
    fn parse_error_multiple() {
        let src = r#"
        var x = @j0Onas;
        var z = 10;
        var y = @b0Onas;
        "#;
        let result = parse_print_error(src);
        let errors = result.err().unwrap();
        assert_eq!(errors.len(), 2);
        let error1 = errors[0].clone();
        assert_matches!(
            error1.kind(),
            ErrorKind::InvalidEndpoint(InvalidEndpointError::InvalidCharacters)
        );
        assert_eq!(error1.span(), Some(17..24));
        let error2 = errors[1].clone();
        assert_matches!(
            error2.kind(),
            ErrorKind::InvalidEndpoint(InvalidEndpointError::InvalidCharacters)
        );
        assert_eq!(error2.span(), Some(62..69));
    }

    #[test]
    fn parse_error_invalid_declaration() {
        let src = "var x = 10; const x += 5;";
        let result = parse_print_error(src);
        let errors = result.err().unwrap();
        assert_eq!(errors.len(), 1);
        let error = errors[0].clone();
        assert_eq!(
            error.message(),
            "Cannot use '+=' operator in variable declaration"
        );
        assert_eq!(error.span(), Some(12..17));
    }

    #[test]
    fn parse_error_u8() {
        let src = "var x = 256u8;";
        let result = parse_print_error(src);
        let errors = result.err().unwrap();
        assert_eq!(errors.len(), 1);
        let error = errors[0].clone();
        assert_eq!(
            error.message(),
            "The number is out of range for the specified type."
        );
        assert_eq!(error.span(), Some(8..13));
    }

    #[test]
    fn parse_error_typed_decimal() {
        let src: &'static str =
            "var x = 10000000000000000000000000000000000000000000000000.3f32";
        let result = parse_print_error(src);

        let errors = result.err().unwrap();
        assert_eq!(errors.len(), 1);
        let error = errors[0].clone();
        assert_eq!(
            error.message(),
            "The number is out of range for the specified type."
        );
        assert_eq!(error.span(), Some(8..63));
    }

    #[test]
    fn function_simple() {
        let src = r#"
            function myFunction() (
                42
            )
        "#;
        let val = parse_unwrap_data(src);
        assert_eq!(
            val,
            DatexExpressionData::FunctionDeclaration(FunctionDeclaration {
                name: "myFunction".to_string(),
                parameters: Vec::new(),
                return_type: None,
                body: Box::new(
                    DatexExpressionData::Integer(Integer::from(42))
                        .with_default_span()
                ),
            })
        );
    }

    #[test]
    fn function_with_params() {
        let src = r#"
            function myFunction(x: integer) (
                42
            )
        "#;
        let val = parse_unwrap_data(src);
        assert_eq!(
            val,
            DatexExpressionData::FunctionDeclaration(FunctionDeclaration {
                name: "myFunction".to_string(),
                parameters: vec![(
                    "x".to_string(),
                    TypeExpressionData::Literal("integer".to_owned())
                        .with_default_span()
                )],
                return_type: None,
                body: Box::new(
                    DatexExpressionData::Integer(Integer::from(42))
                        .with_default_span()
                ),
            })
        );

        let src = r#"
            function myFunction(x: integer, y: integer) (
                1 + 2;
            )
        "#;
        let val = parse_unwrap_data(src);
        assert_eq!(
            val,
            DatexExpressionData::FunctionDeclaration(FunctionDeclaration {
                name: "myFunction".to_string(),
                parameters: vec![
                    (
                        "x".to_string(),
                        TypeExpressionData::Literal("integer".to_owned())
                            .with_default_span()
                    ),
                    (
                        "y".to_string(),
                        TypeExpressionData::Literal("integer".to_owned())
                            .with_default_span()
                    )
                ],
                return_type: None,
                body: Box::new(
                    DatexExpressionData::Statements(
                        Statements::new_terminated(vec![
                            DatexExpressionData::BinaryOperation(
                                BinaryOperation {
                                    operator: BinaryOperator::Arithmetic(
                                        ArithmeticOperator::Add
                                    ),
                                    left: Box::new(
                                        DatexExpressionData::Integer(
                                            Integer::from(1)
                                        )
                                        .with_default_span()
                                    ),
                                    right: Box::new(
                                        DatexExpressionData::Integer(
                                            Integer::from(2)
                                        )
                                        .with_default_span()
                                    ),
                                    r#type: None
                                }
                            )
                            .with_default_span()
                        ])
                    )
                    .with_default_span()
                ),
            })
        );
    }

    #[test]
    fn test_function_with_return_type() {
        let src = r#"
            function myFunction(x: integer) -> integer | text (
                42
            )
        "#;
        let val = parse_unwrap_data(src);
        assert_eq!(
            val,
            DatexExpressionData::FunctionDeclaration(FunctionDeclaration {
                name: "myFunction".to_string(),
                parameters: vec![(
                    "x".to_string(),
                    TypeExpressionData::Literal("integer".to_owned())
                        .with_default_span()
                ),],
                return_type: Some(
                    TypeExpressionData::Union(Union(vec![
                        TypeExpressionData::Literal("integer".to_owned())
                            .with_default_span(),
                        TypeExpressionData::Literal("text".to_owned())
                            .with_default_span()
                    ]))
                    .with_default_span()
                ),
                body: Box::new(
                    DatexExpressionData::Integer(Integer::from(42))
                        .with_default_span()
                ),
            })
        );
    }

    #[test]
    fn type_var_declaration() {
        let src = "var x: 5 = 42";
        let val = parse_unwrap_data(src);
        assert_eq!(
            val,
            DatexExpressionData::VariableDeclaration(VariableDeclaration {
                id: None,
                kind: VariableKind::Var,
                type_annotation: Some(
                    TypeExpressionData::Integer(Integer::from(5))
                        .with_default_span()
                ),
                name: "x".to_string(),
                init_expression: Box::new(
                    DatexExpressionData::Integer(Integer::from(42))
                        .with_default_span()
                )
            })
        );

        let src = "var x: integer/u8 = 42";
        let val = parse_unwrap_data(src);
        assert_eq!(
            val,
            DatexExpressionData::VariableDeclaration(VariableDeclaration {
                id: None,
                kind: VariableKind::Var,
                type_annotation: Some(
                    TypeExpressionData::Literal("integer/u8".to_owned())
                        .with_default_span()
                ),
                name: "x".to_string(),
                init_expression: Box::new(
                    DatexExpressionData::Integer(Integer::from(42))
                        .with_default_span()
                )
            })
        );
    }

    #[deprecated(note = "Remove intersection from value syntax")]
    #[test]
    fn intersection() {
        let src = "5 & 6";
        let val = parse_unwrap_data(src);
        assert_eq!(
            val,
            DatexExpressionData::BinaryOperation(BinaryOperation {
                operator: BinaryOperator::Bitwise(BitwiseOperator::And),
                left: Box::new(
                    DatexExpressionData::Integer(Integer::from(5))
                        .with_default_span()
                ),
                right: Box::new(
                    DatexExpressionData::Integer(Integer::from(6))
                        .with_default_span()
                ),
                r#type: None
            })
        );

        let src = "(integer/u8 & 6) & 2";
        let val = parse_unwrap_data(src);
        assert_eq!(
            val,
            DatexExpressionData::BinaryOperation(BinaryOperation {
                operator: BinaryOperator::Bitwise(BitwiseOperator::And),
                left: Box::new(
                    DatexExpressionData::BinaryOperation(BinaryOperation {
                        operator: BinaryOperator::Bitwise(BitwiseOperator::And),
                        left: Box::new(
                            DatexExpressionData::BinaryOperation(
                                BinaryOperation {
                                    operator: BinaryOperator::VariantAccess,
                                    left: Box::new(
                                        DatexExpressionData::Identifier(
                                            "integer".to_owned()
                                        )
                                        .with_default_span()
                                    ),
                                    right: Box::new(
                                        DatexExpressionData::Identifier(
                                            "u8".to_owned()
                                        )
                                        .with_default_span()
                                    ),
                                    r#type: None
                                }
                            )
                            .with_default_span()
                        ),
                        right: Box::new(
                            DatexExpressionData::Integer(Integer::from(6))
                                .with_default_span()
                        ),
                        r#type: None
                    })
                    .with_default_span()
                ),
                right: Box::new(
                    DatexExpressionData::Integer(Integer::from(2))
                        .with_default_span()
                ),
                r#type: None
            })
        );
    }

    #[deprecated(note = "Remove union from value syntax")]
    #[test]
    fn union() {
        let src = "5 | 6";
        let val = parse_unwrap_data(src);
        assert_eq!(
            val,
            DatexExpressionData::BinaryOperation(BinaryOperation {
                operator: BinaryOperator::Bitwise(BitwiseOperator::Or),
                left: Box::new(
                    DatexExpressionData::Integer(Integer::from(5))
                        .with_default_span()
                ),
                right: Box::new(
                    DatexExpressionData::Integer(Integer::from(6))
                        .with_default_span()
                ),
                r#type: None
            })
        );

        let src = "(integer/u8 | 6) | 2";
        let val = parse_unwrap_data(src);
        assert_eq!(
            val,
            DatexExpressionData::BinaryOperation(BinaryOperation {
                operator: BinaryOperator::Bitwise(BitwiseOperator::Or),
                left: Box::new(
                    DatexExpressionData::BinaryOperation(BinaryOperation {
                        operator: BinaryOperator::Bitwise(BitwiseOperator::Or),
                        left: Box::new(
                            DatexExpressionData::BinaryOperation(
                                BinaryOperation {
                                    operator: BinaryOperator::VariantAccess,
                                    left: Box::new(
                                        DatexExpressionData::Identifier(
                                            "integer".to_owned()
                                        )
                                        .with_default_span()
                                    ),
                                    right: Box::new(
                                        DatexExpressionData::Identifier(
                                            "u8".to_owned()
                                        )
                                        .with_default_span()
                                    ),
                                    r#type: None
                                }
                            )
                            .with_default_span()
                        ),
                        right: Box::new(
                            DatexExpressionData::Integer(Integer::from(6))
                                .with_default_span()
                        ),
                        r#type: None
                    })
                    .with_default_span()
                ),
                right: Box::new(
                    DatexExpressionData::Integer(Integer::from(2))
                        .with_default_span()
                ),
                r#type: None
            })
        );
    }

    #[test]
    fn binary_operator_precedence() {
        let src = "1 + 2 * 3";
        let val = parse_unwrap_data(src);
        assert_eq!(
            val,
            DatexExpressionData::BinaryOperation(BinaryOperation {
                operator: BinaryOperator::Arithmetic(ArithmeticOperator::Add),
                left: Box::new(
                    DatexExpressionData::Integer(Integer::from(1))
                        .with_default_span()
                ),
                right: Box::new(
                    DatexExpressionData::BinaryOperation(BinaryOperation {
                        operator: BinaryOperator::Arithmetic(
                            ArithmeticOperator::Multiply
                        ),
                        left: Box::new(
                            DatexExpressionData::Integer(Integer::from(2))
                                .with_default_span()
                        ),
                        right: Box::new(
                            DatexExpressionData::Integer(Integer::from(3))
                                .with_default_span()
                        ),
                        r#type: None
                    })
                    .with_default_span()
                ),
                r#type: None
            })
        );

        let src = "1 + 2 & 3";
        let val = parse_unwrap_data(src);
        assert_eq!(
            val,
            DatexExpressionData::BinaryOperation(BinaryOperation {
                operator: BinaryOperator::Bitwise(BitwiseOperator::And),
                left: Box::new(
                    DatexExpressionData::BinaryOperation(BinaryOperation {
                        operator: BinaryOperator::Arithmetic(
                            ArithmeticOperator::Add
                        ),
                        left: Box::new(
                            DatexExpressionData::Integer(Integer::from(1))
                                .with_default_span()
                        ),
                        right: Box::new(
                            DatexExpressionData::Integer(Integer::from(2))
                                .with_default_span()
                        ),
                        r#type: None
                    })
                    .with_default_span()
                ),
                right: Box::new(
                    DatexExpressionData::Integer(Integer::from(3))
                        .with_default_span()
                ),
                r#type: None
            })
        );

        let src = "1 + 2 | 3";
        let val = parse_unwrap_data(src);
        assert_eq!(
            val,
            DatexExpressionData::BinaryOperation(BinaryOperation {
                operator: BinaryOperator::Bitwise(BitwiseOperator::Or),
                left: Box::new(
                    DatexExpressionData::BinaryOperation(BinaryOperation {
                        operator: BinaryOperator::Arithmetic(
                            ArithmeticOperator::Add
                        ),
                        left: Box::new(
                            DatexExpressionData::Integer(Integer::from(1))
                                .with_default_span()
                        ),
                        right: Box::new(
                            DatexExpressionData::Integer(Integer::from(2))
                                .with_default_span()
                        ),
                        r#type: None
                    })
                    .with_default_span()
                ),
                right: Box::new(
                    DatexExpressionData::Integer(Integer::from(3))
                        .with_default_span()
                ),
                r#type: None
            })
        );
    }

    #[test]
    fn generic_assessor() {
        let expected = DatexExpressionData::ApplyChain(ApplyChain {
            base: Box::new(
                DatexExpressionData::Identifier("User".to_string())
                    .with_default_span(),
            ),
            operations: vec![
                ApplyOperation::GenericAccess(
                    DatexExpressionData::BinaryOperation(BinaryOperation {
                        operator: BinaryOperator::VariantAccess,
                        left: Box::new(
                            DatexExpressionData::Identifier(
                                "integer".to_owned(),
                            )
                            .with_default_span(),
                        ),
                        right: Box::new(
                            DatexExpressionData::Identifier("u8".to_owned())
                                .with_default_span(),
                        ),
                        r#type: None,
                    })
                    .with_default_span(),
                ),
                ApplyOperation::FunctionCall(
                    DatexExpressionData::Map(Map::new(vec![]))
                        .with_default_span(),
                ),
            ],
        });
        assert_eq!(parse_unwrap_data("User<integer/u8> {}"), expected);
        assert_eq!(parse_unwrap_data("User< integer/u8 > {}"), expected);
        assert_eq!(parse_unwrap_data("User<integer/u8 > {}"), expected);
        assert!(!parse("User <integer/u8> {}").is_valid());
    }

    #[test]
    fn if_else() {
        let src = vec![
            "if true (1) else (2)",
            "if true 1 else 2",
            "if (true) (1) else (2)",
            "if (true) 1 else 2",
            "if true (1) else 2",
            "if (true) 1 else (2)",
            "if true 1 else (2)",
        ];
        for s in src {
            let val = parse_unwrap_data(s);
            assert_eq!(
                val,
                DatexExpressionData::Conditional(Conditional {
                    condition: Box::new(
                        DatexExpressionData::Boolean(true).with_default_span()
                    ),
                    then_branch: Box::new(
                        DatexExpressionData::Integer(Integer::from(1))
                            .with_default_span()
                    ),
                    else_branch: Some(Box::new(
                        DatexExpressionData::Integer(Integer::from(2))
                            .with_default_span()
                    )),
                })
            );
        }

        let src = vec![
            "if true + 1 == 2 (4) else 2",
            "if (true + 1) == 2 4 else 2",
            "if true + 1 == 2 (4) else (2)",
            "if (true + 1) == 2 (4) else (2)",
            "if true + 1 == 2 (4) else 2",
            "if (true + 1) == 2 4 else (2)",
        ];
        for s in src {
            println!("{}", s);
            let val = parse_unwrap_data(s);
            assert_eq!(
                val,
                DatexExpressionData::Conditional(Conditional {
                    condition: Box::new(
                        DatexExpressionData::ComparisonOperation(
                            ComparisonOperation {
                                operator: ComparisonOperator::StructuralEqual,
                                left: Box::new(
                                    DatexExpressionData::BinaryOperation(
                                        BinaryOperation {
                                            operator:
                                                BinaryOperator::Arithmetic(
                                                    ArithmeticOperator::Add
                                                ),
                                            left: Box::new(
                                                DatexExpressionData::Boolean(
                                                    true
                                                )
                                                .with_default_span()
                                            ),
                                            right: Box::new(
                                                DatexExpressionData::Integer(
                                                    Integer::from(1)
                                                )
                                                .with_default_span()
                                            ),
                                            r#type: None
                                        }
                                    )
                                    .with_default_span()
                                ),
                                right: Box::new(
                                    DatexExpressionData::Integer(
                                        Integer::from(2)
                                    )
                                    .with_default_span()
                                )
                            }
                        )
                        .with_default_span()
                    ),
                    then_branch: Box::new(
                        DatexExpressionData::Integer(Integer::from(4))
                            .with_default_span()
                    ),
                    else_branch: Some(Box::new(
                        DatexExpressionData::Integer(Integer::from(2))
                            .with_default_span()
                    )),
                })
            );
        }

        // make sure apply chains still work
        let src = vec![
            "if true + 1 == 2 test [1,2,3]",
            "if true + 1 == 2 (test [1,2,3])",
        ];
        for s in src {
            let val = parse_unwrap_data(s);
            assert_eq!(
                val,
                DatexExpressionData::Conditional(Conditional {
                    condition: Box::new(
                        DatexExpressionData::ComparisonOperation(
                            ComparisonOperation {
                                operator: ComparisonOperator::StructuralEqual,
                                left: Box::new(
                                    DatexExpressionData::BinaryOperation(
                                        BinaryOperation {
                                            operator:
                                                BinaryOperator::Arithmetic(
                                                    ArithmeticOperator::Add
                                                ),
                                            left: Box::new(
                                                DatexExpressionData::Boolean(
                                                    true
                                                )
                                                .with_default_span()
                                            ),
                                            right: Box::new(
                                                DatexExpressionData::Integer(
                                                    Integer::from(1)
                                                )
                                                .with_default_span()
                                            ),
                                            r#type: None
                                        }
                                    )
                                    .with_default_span()
                                ),
                                right: Box::new(
                                    DatexExpressionData::Integer(
                                        Integer::from(2)
                                    )
                                    .with_default_span()
                                )
                            }
                        )
                        .with_default_span()
                    ),
                    then_branch: Box::new(
                        DatexExpressionData::ApplyChain(ApplyChain {
                            base: Box::new(
                                DatexExpressionData::Identifier(
                                    "test".to_string()
                                )
                                .with_default_span()
                            ),
                            operations: vec![ApplyOperation::FunctionCall(
                                DatexExpressionData::List(List::new(vec![
                                    DatexExpressionData::Integer(
                                        Integer::from(1)
                                    )
                                    .with_default_span(),
                                    DatexExpressionData::Integer(
                                        Integer::from(2)
                                    )
                                    .with_default_span(),
                                    DatexExpressionData::Integer(
                                        Integer::from(3)
                                    )
                                    .with_default_span(),
                                ]))
                                .with_default_span()
                            )]
                        })
                        .with_default_span()
                    ),
                    else_branch: None,
                })
            );
        }
    }

    #[test]
    fn if_else_if_else() {
        let src = r#"
            if x == 4 (
                "4"
            ) else if x == 'hello' (
                "42" 
            ) else null
        "#;

        let val = parse_unwrap_data(src);
        assert_eq!(
            val,
            DatexExpressionData::Conditional(Conditional {
                condition: Box::new(
                    DatexExpressionData::ComparisonOperation(
                        ComparisonOperation {
                            operator: ComparisonOperator::StructuralEqual,
                            left: Box::new(
                                DatexExpressionData::Identifier(
                                    "x".to_string()
                                )
                                .with_default_span()
                            ),
                            right: Box::new(
                                DatexExpressionData::Integer(Integer::from(4))
                                    .with_default_span()
                            )
                        }
                    )
                    .with_default_span()
                ),
                then_branch: Box::new(
                    DatexExpressionData::Text("4".to_string())
                        .with_default_span()
                ),
                else_branch: Some(Box::new(
                    DatexExpressionData::Conditional(Conditional {
                        condition: Box::new(
                            DatexExpressionData::ComparisonOperation(
                                ComparisonOperation {
                                    operator:
                                        ComparisonOperator::StructuralEqual,
                                    left: Box::new(
                                        DatexExpressionData::Identifier(
                                            "x".to_string()
                                        )
                                        .with_default_span()
                                    ),
                                    right: Box::new(
                                        DatexExpressionData::Text(
                                            "hello".to_string()
                                        )
                                        .with_default_span()
                                    )
                                }
                            )
                            .with_default_span()
                        ),
                        then_branch: Box::new(
                            DatexExpressionData::Text("42".to_string())
                                .with_default_span()
                        ),
                        else_branch: Some(Box::new(
                            DatexExpressionData::Null.with_default_span()
                        ))
                    })
                    .with_default_span()
                )),
            })
        );
    }

    #[test]
    fn unary_operator() {
        let src = "+(User {})";
        let val = parse_unwrap_data(src);
        assert_eq!(
            val,
            DatexExpressionData::UnaryOperation(UnaryOperation {
                operator: UnaryOperator::Arithmetic(
                    ArithmeticUnaryOperator::Plus
                ),
                expression: Box::new(
                    DatexExpressionData::ApplyChain(ApplyChain {
                        base: Box::new(
                            DatexExpressionData::Identifier("User".to_string())
                                .with_default_span()
                        ),
                        operations: vec![ApplyOperation::FunctionCall(
                            DatexExpressionData::Map(Map::new(vec![]))
                                .with_default_span()
                        )]
                    })
                    .with_default_span()
                ),
            })
        );

        let src = "-(5)";
        let val = parse_unwrap_data(src);
        assert_eq!(
            val,
            DatexExpressionData::UnaryOperation(UnaryOperation {
                operator: UnaryOperator::Arithmetic(
                    ArithmeticUnaryOperator::Minus
                ),
                expression: Box::new(
                    DatexExpressionData::Integer(Integer::from(5))
                        .with_default_span()
                )
            })
        );

        let src = "+-+-myVal";
        let val = parse_unwrap_data(src);
        assert_eq!(
            val,
            DatexExpressionData::UnaryOperation(UnaryOperation {
                operator: UnaryOperator::Arithmetic(ArithmeticUnaryOperator::Plus),
                expression: Box::new(DatexExpressionData::UnaryOperation(UnaryOperation {
                    operator: UnaryOperator::Arithmetic(ArithmeticUnaryOperator::Minus),
                    expression: Box::new(DatexExpressionData::UnaryOperation(UnaryOperation {
                        operator: UnaryOperator::Arithmetic(
                            ArithmeticUnaryOperator::Plus
                        ),
                        expression: Box::new(DatexExpressionData::UnaryOperation(UnaryOperation {
                            operator: UnaryOperator::Arithmetic(
                                ArithmeticUnaryOperator::Minus
                            ),
                            expression: Box::new(DatexExpressionData::Identifier(
                                "myVal".to_string()
                            ).with_default_span())
                        }).with_default_span())
                    }).with_default_span())
                }).with_default_span())
            })
        );
    }

    #[test]
    fn var_declaration_with_type_simple() {
        let src = "var x: integer = 42";
        let val = parse_unwrap_data(src);
        assert_eq!(
            val,
            DatexExpressionData::VariableDeclaration(VariableDeclaration {
                id: None,
                kind: VariableKind::Var,
                type_annotation: Some(
                    TypeExpressionData::Literal("integer".to_string())
                        .with_default_span()
                ),
                name: "x".to_string(),
                init_expression: Box::new(
                    DatexExpressionData::Integer(Integer::from(42))
                        .with_default_span()
                )
            })
        );

        let src = "var x: User = 42";
        let val = parse_unwrap_data(src);
        assert_eq!(
            val,
            DatexExpressionData::VariableDeclaration(VariableDeclaration {
                id: None,
                kind: VariableKind::Var,
                type_annotation: Some(
                    TypeExpressionData::Literal("User".to_string())
                        .with_default_span()
                ),
                name: "x".to_string(),
                init_expression: Box::new(
                    DatexExpressionData::Integer(Integer::from(42))
                        .with_default_span()
                )
            })
        );

        let src = "var x: integer/u8 = 42";
        let val = parse_unwrap_data(src);
        assert_eq!(
            val,
            DatexExpressionData::VariableDeclaration(VariableDeclaration {
                id: None,
                kind: VariableKind::Var,
                type_annotation: Some(
                    TypeExpressionData::Literal("integer/u8".to_owned())
                        .with_default_span()
                ),
                name: "x".to_string(),
                init_expression: Box::new(
                    DatexExpressionData::Integer(Integer::from(42))
                        .with_default_span()
                )
            })
        );
    }

    #[test]
    fn var_declaration_with_type_union() {
        let src = "var x: integer/u8 | text = 42";
        let val = parse_unwrap_data(src);
        assert_eq!(
            val,
            DatexExpressionData::VariableDeclaration(VariableDeclaration {
                id: None,
                kind: VariableKind::Var,
                type_annotation: Some(
                    TypeExpressionData::Union(Union(vec![
                        TypeExpressionData::Literal("integer/u8".to_owned())
                            .with_default_span(),
                        TypeExpressionData::Literal("text".to_owned())
                            .with_default_span()
                    ]))
                    .with_default_span()
                ),
                name: "x".to_string(),
                init_expression: Box::new(
                    DatexExpressionData::Integer(Integer::from(42))
                        .with_default_span()
                )
            })
        );
    }

    #[test]
    fn var_declaration_with_type_intersection() {
        let src = "var x: 5 & 6 = 42";
        let val = parse_unwrap_data(src);
        assert_eq!(
            val,
            DatexExpressionData::VariableDeclaration(VariableDeclaration {
                id: None,
                kind: VariableKind::Var,
                type_annotation: Some(
                    TypeExpressionData::Intersection(Intersection(vec![
                        TypeExpressionData::Integer(Integer::from(5))
                            .with_default_span(),
                        TypeExpressionData::Integer(Integer::from(6))
                            .with_default_span()
                    ]))
                    .with_default_span()
                ),
                name: "x".to_string(),
                init_expression: Box::new(
                    DatexExpressionData::Integer(Integer::from(42))
                        .with_default_span()
                )
            })
        );
    }

    #[test]
    fn test_type_var_declaration_list() {
        let src = "var x: integer[] = 42";
        let val = parse_unwrap_data(src);
        assert_eq!(
            val,
            DatexExpressionData::VariableDeclaration(VariableDeclaration {
                id: None,
                kind: VariableKind::Var,
                type_annotation: Some(
                    TypeExpressionData::SliceList(SliceList(Box::new(
                        TypeExpressionData::Literal("integer".to_owned())
                            .with_default_span()
                    )))
                    .with_default_span()
                ),
                name: "x".to_string(),
                init_expression: Box::new(
                    DatexExpressionData::Integer(Integer::from(42))
                        .with_default_span()
                )
            })
        );
    }

    #[test]
    fn equal_operators() {
        let src = "3 == 1 + 2";
        let val = parse_unwrap_data(src);
        assert_eq!(
            val,
            DatexExpressionData::ComparisonOperation(ComparisonOperation {
                operator: ComparisonOperator::StructuralEqual,
                left: Box::new(
                    DatexExpressionData::Integer(Integer::from(3))
                        .with_default_span()
                ),
                right: Box::new(
                    DatexExpressionData::BinaryOperation(BinaryOperation {
                        operator: BinaryOperator::Arithmetic(
                            ArithmeticOperator::Add
                        ),
                        left: Box::new(
                            DatexExpressionData::Integer(Integer::from(1))
                                .with_default_span()
                        ),
                        right: Box::new(
                            DatexExpressionData::Integer(Integer::from(2))
                                .with_default_span()
                        ),
                        r#type: None
                    })
                    .with_default_span()
                )
            })
        );

        let src = "3 === 1 + 2";
        let val = parse_unwrap_data(src);
        assert_eq!(
            val,
            DatexExpressionData::ComparisonOperation(ComparisonOperation {
                operator: ComparisonOperator::Equal,
                left: Box::new(
                    DatexExpressionData::Integer(Integer::from(3))
                        .with_default_span()
                ),
                right: Box::new(
                    DatexExpressionData::BinaryOperation(BinaryOperation {
                        operator: BinaryOperator::Arithmetic(
                            ArithmeticOperator::Add
                        ),
                        left: Box::new(
                            DatexExpressionData::Integer(Integer::from(1))
                                .with_default_span()
                        ),
                        right: Box::new(
                            DatexExpressionData::Integer(Integer::from(2))
                                .with_default_span()
                        ),
                        r#type: None
                    })
                    .with_default_span()
                )
            })
        );

        let src = "5 != 1 + 2";
        let val = parse_unwrap_data(src);
        assert_eq!(
            val,
            DatexExpressionData::ComparisonOperation(ComparisonOperation {
                operator: ComparisonOperator::NotStructuralEqual,
                left: Box::new(
                    DatexExpressionData::Integer(Integer::from(5))
                        .with_default_span()
                ),
                right: Box::new(
                    DatexExpressionData::BinaryOperation(BinaryOperation {
                        operator: BinaryOperator::Arithmetic(
                            ArithmeticOperator::Add
                        ),
                        left: Box::new(
                            DatexExpressionData::Integer(Integer::from(1))
                                .with_default_span()
                        ),
                        right: Box::new(
                            DatexExpressionData::Integer(Integer::from(2))
                                .with_default_span()
                        ),
                        r#type: None
                    })
                    .with_default_span()
                )
            })
        );
        let src = "5 !== 1 + 2";
        let val = parse_unwrap_data(src);
        assert_eq!(
            val,
            DatexExpressionData::ComparisonOperation(ComparisonOperation {
                operator: ComparisonOperator::NotEqual,
                left: Box::new(
                    DatexExpressionData::Integer(Integer::from(5))
                        .with_default_span()
                ),
                right: Box::new(
                    DatexExpressionData::BinaryOperation(BinaryOperation {
                        operator: BinaryOperator::Arithmetic(
                            ArithmeticOperator::Add
                        ),
                        left: Box::new(
                            DatexExpressionData::Integer(Integer::from(1))
                                .with_default_span()
                        ),
                        right: Box::new(
                            DatexExpressionData::Integer(Integer::from(2))
                                .with_default_span()
                        ),
                        r#type: None
                    })
                    .with_default_span()
                )
            })
        );

        let src = "5 is 1 + 2";
        let val = parse_unwrap_data(src);
        assert_eq!(
            val,
            DatexExpressionData::ComparisonOperation(ComparisonOperation {
                operator: ComparisonOperator::Is,
                left: Box::new(
                    DatexExpressionData::Integer(Integer::from(5))
                        .with_default_span()
                ),
                right: Box::new(
                    DatexExpressionData::BinaryOperation(BinaryOperation {
                        operator: BinaryOperator::Arithmetic(
                            ArithmeticOperator::Add
                        ),
                        left: Box::new(
                            DatexExpressionData::Integer(Integer::from(1))
                                .with_default_span()
                        ),
                        right: Box::new(
                            DatexExpressionData::Integer(Integer::from(2))
                                .with_default_span()
                        ),
                        r#type: None
                    })
                    .with_default_span()
                )
            })
        );
    }

    #[test]
    fn null() {
        let src = "null";
        let val = parse_unwrap_data(src);
        assert_eq!(val, DatexExpressionData::Null);
    }

    #[test]
    fn boolean() {
        let src_true = "true";
        let val_true = parse_unwrap_data(src_true);
        assert_eq!(val_true, DatexExpressionData::Boolean(true));

        let src_false = "false";
        let val_false = parse_unwrap_data(src_false);
        assert_eq!(val_false, DatexExpressionData::Boolean(false));
    }

    #[test]
    fn integer() {
        let src = "123456789123456789";
        let num = parse_unwrap_data(src);
        assert_eq!(
            num,
            DatexExpressionData::Integer(
                Integer::from_string("123456789123456789").unwrap()
            )
        );
    }

    #[test]
    fn negative_integer() {
        let src = "-123456789123456789";
        let num = parse_unwrap_data(src);
        assert_eq!(
            num,
            DatexExpressionData::UnaryOperation(UnaryOperation {
                operator: UnaryOperator::Arithmetic(
                    ArithmeticUnaryOperator::Minus
                ),
                expression: Box::new(
                    DatexExpressionData::Integer(
                        Integer::from_string("123456789123456789").unwrap()
                    )
                    .with_default_span()
                )
            })
        );
    }

    #[test]
    fn integer_with_underscores() {
        let src = "123_456";
        let num = parse_unwrap_data(src);
        assert_eq!(
            num,
            DatexExpressionData::Integer(
                Integer::from_string("123456").unwrap()
            )
        );
    }

    #[test]
    fn hex_integer() {
        let src = "0x1A2B3C4D5E6F";
        let num = parse_unwrap_data(src);
        assert_eq!(
            num,
            DatexExpressionData::Integer(
                Integer::from_string_radix("1A2B3C4D5E6F", 16).unwrap()
            )
        );
    }

    #[test]
    fn octal_integer() {
        let src = "0o755";
        let num = parse_unwrap_data(src);
        assert_eq!(
            num,
            DatexExpressionData::Integer(
                Integer::from_string_radix("755", 8).unwrap()
            )
        );
    }

    #[test]
    fn binary_integer() {
        let src = "0b101010";
        let num = parse_unwrap_data(src);
        assert_eq!(
            num,
            DatexExpressionData::Integer(
                Integer::from_string_radix("101010", 2).unwrap()
            )
        );
    }

    #[test]
    fn integer_with_exponent() {
        let src = "2e10";
        let num = parse_unwrap_data(src);
        assert_eq!(
            num,
            DatexExpressionData::Decimal(
                Decimal::from_string("20000000000").unwrap()
            )
        );
    }

    #[test]
    fn decimal() {
        let src = "123.456789123456";
        let num = parse_unwrap_data(src);
        assert_eq!(
            num,
            DatexExpressionData::Decimal(
                Decimal::from_string("123.456789123456").unwrap()
            )
        );
    }

    #[test]
    fn decimal_with_separator() {
        let cases = [
            ("123_45_6.789", "123456.789"),
            ("123.443_3434", "123.4433434"),
            ("1_000.000_001", "1000.000001"),
            ("3.14_15e+1_0", "31415000000.0"),
            ("0.0_0_1", "0.001"),
            ("1_000.0", "1000.0"),
        ];

        for (src, expected_str) in cases {
            let num = parse_unwrap_data(src);
            assert_eq!(
                num,
                DatexExpressionData::Decimal(
                    Decimal::from_string(expected_str).unwrap()
                ),
                "Failed to parse: {src}"
            );
        }
    }

    #[test]
    fn negative_decimal() {
        let src = "-123.4";
        let num = parse_unwrap_data(src);
        assert_eq!(
            num,
            DatexExpressionData::UnaryOperation(UnaryOperation {
                operator: UnaryOperator::Arithmetic(
                    ArithmeticUnaryOperator::Minus
                ),
                expression: Box::new(
                    DatexExpressionData::Decimal(
                        Decimal::from_string("123.4").unwrap()
                    )
                    .with_default_span()
                )
            })
        );
    }

    #[test]
    fn decimal_with_exponent() {
        let src = "1.23456789123456e2";
        let num = parse_unwrap_data(src);
        assert_eq!(
            num,
            DatexExpressionData::Decimal(
                Decimal::from_string("123.456789123456").unwrap()
            )
        );
    }

    #[test]
    fn decimal_with_negative_exponent() {
        let src = "1.23456789123456e-2";
        let num = parse_unwrap_data(src);
        assert_eq!(
            num,
            DatexExpressionData::Decimal(
                Decimal::from_string("0.0123456789123456").unwrap()
            )
        );
    }

    #[test]
    fn decimal_with_positive_exponent() {
        let src = "1.23456789123456E+2";
        let num = parse_unwrap_data(src);
        assert_eq!(
            num,
            DatexExpressionData::Decimal(
                Decimal::from_string("123.456789123456").unwrap()
            )
        );
    }

    #[test]
    fn decimal_with_trailing_point() {
        let src = "123.";
        let num = parse_unwrap_data(src);
        assert_eq!(
            num,
            DatexExpressionData::Decimal(
                Decimal::from_string("123.0").unwrap()
            )
        );
    }

    #[test]
    fn decimal_with_leading_point() {
        let src = ".456789123456";
        let num = parse_unwrap_data(src);
        assert_eq!(
            num,
            DatexExpressionData::Decimal(
                Decimal::from_string("0.456789123456").unwrap()
            )
        );

        let src = ".423e-2";
        let num = parse_unwrap_data(src);
        assert_eq!(
            num,
            DatexExpressionData::Decimal(
                Decimal::from_string("0.00423").unwrap()
            )
        );
    }

    #[test]
    fn text_double_quotes() {
        let src = r#""Hello, world!""#;
        let text = parse_unwrap_data(src);
        assert_eq!(
            text,
            DatexExpressionData::Text("Hello, world!".to_string())
        );
    }

    #[test]
    fn text_single_quotes() {
        let src = r#"'Hello, world!'"#;
        let text = parse_unwrap_data(src);
        assert_eq!(
            text,
            DatexExpressionData::Text("Hello, world!".to_string())
        );
    }

    #[test]
    fn text_escape_sequences() {
        let src =
            r#""Hello, \"world\"! \n New line \t tab \uD83D\uDE00 \u2764""#;
        let text = parse_unwrap_data(src);

        assert_eq!(
            text,
            DatexExpressionData::Text(
                "Hello, \"world\"! \n New line \t tab 😀 ❤".to_string()
            )
        );
    }

    #[test]
    fn text_escape_sequences_2() {
        let src =
            r#""\u0048\u0065\u006C\u006C\u006F, \u2764\uFE0F, \uD83D\uDE00""#;
        let text = parse_unwrap_data(src);
        assert_eq!(
            text,
            DatexExpressionData::Text("Hello, ❤️, 😀".to_string())
        );
    }

    #[test]
    fn text_nested_escape_sequences() {
        let src = r#""\\\\""#;
        let text = parse_unwrap_data(src);
        assert_eq!(text, DatexExpressionData::Text("\\\\".to_string()));
    }

    #[test]
    fn text_nested_escape_sequences_2() {
        let src = r#""\\\"""#;
        let text = parse_unwrap_data(src);
        assert_eq!(text, DatexExpressionData::Text("\\\"".to_string()));
    }

    #[test]
    fn empty_list() {
        let src = "[]";
        let arr = parse_unwrap_data(src);
        assert_eq!(arr, DatexExpressionData::List(List::new(vec![])));
    }

    #[test]
    fn list_with_values() {
        let src = "[1, 2, 3, 4.5, \"text\"]";
        let arr = parse_unwrap_data(src);

        assert_eq!(
            arr,
            DatexExpressionData::List(List::new(vec![
                DatexExpressionData::Integer(Integer::from(1))
                    .with_default_span(),
                DatexExpressionData::Integer(Integer::from(2))
                    .with_default_span(),
                DatexExpressionData::Integer(Integer::from(3))
                    .with_default_span(),
                DatexExpressionData::Decimal(
                    Decimal::from_string("4.5").unwrap()
                )
                .with_default_span(),
                DatexExpressionData::Text("text".to_string())
                    .with_default_span(),
            ]))
        );
    }

    #[test]
    fn empty_map() {
        let src = "{}";
        let obj = parse_unwrap_data(src);

        assert_eq!(obj, DatexExpressionData::Map(Map::new(vec![])));
    }

    #[test]
    fn list_of_lists() {
        let src = "[[1,2],3,[4]]";
        let arr = parse_unwrap_data(src);

        assert_eq!(
            arr,
            DatexExpressionData::List(List::new(vec![
                DatexExpressionData::List(List::new(vec![
                    DatexExpressionData::Integer(Integer::from(1))
                        .with_default_span(),
                    DatexExpressionData::Integer(Integer::from(2))
                        .with_default_span(),
                ]))
                .with_default_span(),
                DatexExpressionData::Integer(Integer::from(3))
                    .with_default_span(),
                DatexExpressionData::List(List::new(vec![
                    DatexExpressionData::Integer(Integer::from(4))
                        .with_default_span()
                ]))
                .with_default_span(),
            ]))
        );
    }

    #[test]
    fn single_entry_map() {
        let src = "{x: 1}";
        let map = parse_unwrap_data(src);
        assert_eq!(
            map,
            DatexExpressionData::Map(Map::new(vec![(
                DatexExpressionData::Text("x".to_string()).with_default_span(),
                DatexExpressionData::Integer(Integer::from(1))
                    .with_default_span()
            )]))
        );
    }

    #[test]
    fn scoped_atom() {
        let src = "(1)";
        let atom = parse_unwrap_data(src);
        assert_eq!(atom, DatexExpressionData::Integer(Integer::from(1)));
    }

    #[test]
    fn scoped_list() {
        let src = "(([1, 2, 3]))";
        let arr = parse_unwrap_data(src);

        assert_eq!(
            arr,
            DatexExpressionData::List(List::new(vec![
                DatexExpressionData::Integer(Integer::from(1))
                    .with_default_span(),
                DatexExpressionData::Integer(Integer::from(2))
                    .with_default_span(),
                DatexExpressionData::Integer(Integer::from(3))
                    .with_default_span(),
            ]))
        );
    }

    #[test]
    fn map_with_key_value_pairs() {
        let src = r#"{"key1": "value1", "key2": 42, "key3": true}"#;
        let obj = parse_unwrap_data(src);

        assert_eq!(
            obj,
            DatexExpressionData::Map(Map::new(vec![
                (
                    DatexExpressionData::Text("key1".to_string())
                        .with_default_span(),
                    DatexExpressionData::Text("value1".to_string())
                        .with_default_span()
                ),
                (
                    DatexExpressionData::Text("key2".to_string())
                        .with_default_span(),
                    DatexExpressionData::Integer(Integer::from(42))
                        .with_default_span()
                ),
                (
                    DatexExpressionData::Text("key3".to_string())
                        .with_default_span(),
                    DatexExpressionData::Boolean(true).with_default_span()
                ),
            ]))
        );
    }

    #[test]
    fn dynamic_map_keys() {
        let src = r#"{(1): "value1", (2): 42, (3): true}"#;
        let obj = parse_unwrap_data(src);
        assert_eq!(
            obj,
            DatexExpressionData::Map(Map::new(vec![
                (
                    DatexExpressionData::Integer(Integer::from(1))
                        .with_default_span(),
                    DatexExpressionData::Text("value1".to_string())
                        .with_default_span()
                ),
                (
                    DatexExpressionData::Integer(Integer::from(2))
                        .with_default_span(),
                    DatexExpressionData::Integer(Integer::from(42))
                        .with_default_span()
                ),
                (
                    DatexExpressionData::Integer(Integer::from(3))
                        .with_default_span(),
                    DatexExpressionData::Boolean(true).with_default_span()
                ),
            ]))
        );
    }

    #[test]
    fn add() {
        // Test with escaped characters in text
        let src = "1 + 2";
        let expr = parse_unwrap_data(src);
        assert_eq!(
            expr,
            DatexExpressionData::BinaryOperation(BinaryOperation {
                operator: BinaryOperator::Arithmetic(ArithmeticOperator::Add),
                left: Box::new(
                    DatexExpressionData::Integer(Integer::from(1))
                        .with_default_span()
                ),
                right: Box::new(
                    DatexExpressionData::Integer(Integer::from(2))
                        .with_default_span()
                ),
                r#type: None
            })
        );
    }

    #[test]
    fn add_complex_values() {
        // Test with escaped characters in text
        let src = "[] + x + (1 + 2)";
        let expr = parse_unwrap_data(src);
        assert_eq!(
            expr,
            DatexExpressionData::BinaryOperation(BinaryOperation {
                operator: BinaryOperator::Arithmetic(ArithmeticOperator::Add),
                left: Box::new(
                    DatexExpressionData::BinaryOperation(BinaryOperation {
                        operator: BinaryOperator::Arithmetic(
                            ArithmeticOperator::Add
                        ),
                        left: Box::new(
                            DatexExpressionData::List(List::new(vec![]))
                                .with_default_span()
                        ),
                        right: Box::new(
                            DatexExpressionData::Identifier("x".to_string())
                                .with_default_span()
                        ),
                        r#type: None
                    })
                    .with_default_span()
                ),
                right: Box::new(
                    DatexExpressionData::BinaryOperation(BinaryOperation {
                        operator: BinaryOperator::Arithmetic(
                            ArithmeticOperator::Add
                        ),
                        left: Box::new(
                            DatexExpressionData::Integer(Integer::from(1))
                                .with_default_span()
                        ),
                        right: Box::new(
                            DatexExpressionData::Integer(Integer::from(2))
                                .with_default_span()
                        ),
                        r#type: None
                    })
                    .with_default_span()
                ),
                r#type: None
            })
        );
    }

    #[test]
    fn subtract() {
        let src = "5 - 3";
        let expr = parse_unwrap_data(src);
        assert_eq!(
            expr,
            DatexExpressionData::BinaryOperation(BinaryOperation {
                operator: BinaryOperator::Arithmetic(
                    ArithmeticOperator::Subtract
                ),
                left: Box::new(
                    DatexExpressionData::Integer(Integer::from(5))
                        .with_default_span()
                ),
                right: Box::new(
                    DatexExpressionData::Integer(Integer::from(3))
                        .with_default_span()
                ),
                r#type: None
            })
        );

        let src = "5-3";
        let expr = parse_unwrap_data(src);
        assert_eq!(
            expr,
            DatexExpressionData::BinaryOperation(BinaryOperation {
                operator: BinaryOperator::Arithmetic(
                    ArithmeticOperator::Subtract
                ),
                left: Box::new(
                    DatexExpressionData::Integer(Integer::from(5))
                        .with_default_span()
                ),
                right: Box::new(
                    DatexExpressionData::Integer(Integer::from(3))
                        .with_default_span()
                ),
                r#type: None
            })
        );

        let src = "5- 3";
        let expr = parse_unwrap_data(src);
        assert_eq!(
            expr,
            DatexExpressionData::BinaryOperation(BinaryOperation {
                operator: BinaryOperator::Arithmetic(
                    ArithmeticOperator::Subtract
                ),
                left: Box::new(
                    DatexExpressionData::Integer(Integer::from(5))
                        .with_default_span()
                ),
                right: Box::new(
                    DatexExpressionData::Integer(Integer::from(3))
                        .with_default_span()
                ),
                r#type: None
            })
        );

        let src = "5 -3";
        let expr = parse_unwrap_data(src);
        assert_eq!(
            expr,
            DatexExpressionData::BinaryOperation(BinaryOperation {
                operator: BinaryOperator::Arithmetic(
                    ArithmeticOperator::Subtract
                ),
                left: Box::new(
                    DatexExpressionData::Integer(Integer::from(5))
                        .with_default_span()
                ),
                right: Box::new(
                    DatexExpressionData::Integer(Integer::from(3))
                        .with_default_span()
                ),
                r#type: None
            })
        );
    }

    #[test]
    fn multiply() {
        let src = "4 * 2";
        let expr = parse_unwrap_data(src);
        assert_eq!(
            expr,
            DatexExpressionData::BinaryOperation(BinaryOperation {
                operator: BinaryOperator::Arithmetic(
                    ArithmeticOperator::Multiply
                ),
                left: Box::new(
                    DatexExpressionData::Integer(Integer::from(4))
                        .with_default_span()
                ),
                right: Box::new(
                    DatexExpressionData::Integer(Integer::from(2))
                        .with_default_span()
                ),
                r#type: None
            })
        );
    }

    #[test]
    fn divide() {
        let src = "8 / 2";
        let expr = parse_unwrap_data(src);
        assert_eq!(
            expr,
            DatexExpressionData::BinaryOperation(BinaryOperation {
                operator: BinaryOperator::Arithmetic(
                    ArithmeticOperator::Divide
                ),
                left: Box::new(
                    DatexExpressionData::Integer(Integer::from(8))
                        .with_default_span()
                ),
                right: Box::new(
                    DatexExpressionData::Integer(Integer::from(2))
                        .with_default_span()
                ),
                r#type: None
            })
        );

        let src = "8 /2";
        let expr = parse_unwrap_data(src);
        assert_eq!(
            expr,
            DatexExpressionData::BinaryOperation(BinaryOperation {
                operator: BinaryOperator::Arithmetic(
                    ArithmeticOperator::Divide
                ),
                left: Box::new(
                    DatexExpressionData::Integer(Integer::from(8))
                        .with_default_span()
                ),
                right: Box::new(
                    DatexExpressionData::Integer(Integer::from(2))
                        .with_default_span()
                ),
                r#type: None
            })
        );

        let src = "8u8/2";
        let expr = parse_unwrap_data(src);
        assert_eq!(
            expr,
            DatexExpressionData::BinaryOperation(BinaryOperation {
                operator: BinaryOperator::Arithmetic(
                    ArithmeticOperator::Divide
                ),
                left: Box::new(
                    DatexExpressionData::TypedInteger(TypedInteger::from(8u8))
                        .with_default_span()
                ),
                right: Box::new(
                    DatexExpressionData::Integer(Integer::from(2))
                        .with_default_span()
                ),
                r#type: None
            })
        );
    }

    #[test]
    fn complex_calculation() {
        let src = "1 + 2 * 3 + 4";
        let expr = parse_unwrap_data(src);
        assert_eq!(
            expr,
            DatexExpressionData::BinaryOperation(BinaryOperation {
                operator: BinaryOperator::Arithmetic(ArithmeticOperator::Add),
                left: Box::new(
                    DatexExpressionData::BinaryOperation(BinaryOperation {
                        operator: BinaryOperator::Arithmetic(
                            ArithmeticOperator::Add
                        ),
                        left: Box::new(
                            DatexExpressionData::Integer(Integer::from(1))
                                .with_default_span()
                        ),
                        right: Box::new(
                            DatexExpressionData::BinaryOperation(
                                BinaryOperation {
                                    operator: BinaryOperator::Arithmetic(
                                        ArithmeticOperator::Multiply
                                    ),
                                    left: Box::new(
                                        DatexExpressionData::Integer(
                                            Integer::from(2)
                                        )
                                        .with_default_span()
                                    ),
                                    right: Box::new(
                                        DatexExpressionData::Integer(
                                            Integer::from(3)
                                        )
                                        .with_default_span()
                                    ),
                                    r#type: None
                                }
                            )
                            .with_default_span()
                        ),
                        r#type: None
                    })
                    .with_default_span()
                ),
                right: Box::new(
                    DatexExpressionData::Integer(Integer::from(4))
                        .with_default_span()
                ),
                r#type: None
            })
        );
    }

    #[test]
    fn nested_addition() {
        let src = "1 + (2 + 3)";
        let expr = parse_unwrap_data(src);
        assert_eq!(
            expr,
            DatexExpressionData::BinaryOperation(BinaryOperation {
                operator: BinaryOperator::Arithmetic(ArithmeticOperator::Add),
                left: Box::new(
                    DatexExpressionData::Integer(Integer::from(1))
                        .with_default_span()
                ),
                right: Box::new(
                    DatexExpressionData::BinaryOperation(BinaryOperation {
                        operator: BinaryOperator::Arithmetic(
                            ArithmeticOperator::Add
                        ),
                        left: Box::new(
                            DatexExpressionData::Integer(Integer::from(2))
                                .with_default_span()
                        ),
                        right: Box::new(
                            DatexExpressionData::Integer(Integer::from(3))
                                .with_default_span()
                        ),
                        r#type: None
                    })
                    .with_default_span()
                ),
                r#type: None
            })
        );
    }

    #[test]
    fn add_statements_1() {
        // Test with escaped characters in text
        let src = "1 + (2;3)";
        let expr = parse_unwrap_data(src);
        assert_eq!(
            expr,
            DatexExpressionData::BinaryOperation(BinaryOperation {
                operator: BinaryOperator::Arithmetic(ArithmeticOperator::Add),
                left: Box::new(
                    DatexExpressionData::Integer(Integer::from(1))
                        .with_default_span()
                ),
                right: Box::new(
                    DatexExpressionData::Statements(
                        Statements::new_unterminated(vec![
                            DatexExpressionData::Integer(Integer::from(2))
                                .with_default_span(),
                            DatexExpressionData::Integer(Integer::from(3))
                                .with_default_span(),
                        ])
                    )
                    .with_default_span()
                ),
                r#type: None
            })
        );
    }

    #[test]
    fn add_statements_2() {
        // Test with escaped characters in text
        let src = "(1;2) + 3";
        let expr = parse_unwrap_data(src);
        assert_eq!(
            expr,
            DatexExpressionData::BinaryOperation(BinaryOperation {
                operator: BinaryOperator::Arithmetic(ArithmeticOperator::Add),
                left: Box::new(
                    DatexExpressionData::Statements(
                        Statements::new_unterminated(vec![
                            DatexExpressionData::Integer(Integer::from(1))
                                .with_default_span(),
                            DatexExpressionData::Integer(Integer::from(2))
                                .with_default_span(),
                        ])
                    )
                    .with_default_span()
                ),
                right: Box::new(
                    DatexExpressionData::Integer(Integer::from(3))
                        .with_default_span()
                ),
                r#type: None
            })
        );
    }

    #[test]
    fn nested_expressions() {
        let src = "[1 + 2]";
        let expr = parse_unwrap_data(src);
        assert_eq!(
            expr,
            DatexExpressionData::List(List::new(vec![
                DatexExpressionData::BinaryOperation(BinaryOperation {
                    operator: BinaryOperator::Arithmetic(
                        ArithmeticOperator::Add
                    ),
                    left: Box::new(
                        DatexExpressionData::Integer(Integer::from(1))
                            .with_default_span()
                    ),
                    right: Box::new(
                        DatexExpressionData::Integer(Integer::from(2))
                            .with_default_span()
                    ),
                    r#type: None
                })
                .with_default_span()
            ]))
        );
    }

    #[test]
    fn multi_statement_expression() {
        let src = "1;2";
        let expr = parse_unwrap_data(src);
        assert_eq!(
            expr,
            DatexExpressionData::Statements(Statements::new_unterminated(
                vec![
                    DatexExpressionData::Integer(Integer::from(1))
                        .with_default_span(),
                    DatexExpressionData::Integer(Integer::from(2))
                        .with_default_span(),
                ]
            ))
        );
    }

    #[test]
    fn nested_scope_statements() {
        let src = "(1; 2; 3)";
        let expr = parse_unwrap_data(src);
        assert_eq!(
            expr,
            DatexExpressionData::Statements(Statements::new_unterminated(
                vec![
                    DatexExpressionData::Integer(Integer::from(1))
                        .with_default_span(),
                    DatexExpressionData::Integer(Integer::from(2))
                        .with_default_span(),
                    DatexExpressionData::Integer(Integer::from(3))
                        .with_default_span(),
                ]
            ))
        );
    }
    #[test]
    fn nested_scope_statements_closed() {
        let src = "(1; 2; 3;)";
        let expr = parse_unwrap_data(src);
        assert_eq!(
            expr,
            DatexExpressionData::Statements(Statements::new_terminated(vec![
                DatexExpressionData::Integer(Integer::from(1))
                    .with_default_span(),
                DatexExpressionData::Integer(Integer::from(2))
                    .with_default_span(),
                DatexExpressionData::Integer(Integer::from(3))
                    .with_default_span(),
            ]))
        );
    }

    #[test]
    fn nested_statements_in_map() {
        let src = r#"{"key": (1; 2; 3)}"#;
        let expr = parse_unwrap_data(src);
        assert_eq!(
            expr,
            DatexExpressionData::Map(Map::new(vec![(
                DatexExpressionData::Text("key".to_string())
                    .with_default_span(),
                DatexExpressionData::Statements(Statements::new_unterminated(
                    vec![
                        DatexExpressionData::Integer(Integer::from(1))
                            .with_default_span(),
                        DatexExpressionData::Integer(Integer::from(2))
                            .with_default_span(),
                        DatexExpressionData::Integer(Integer::from(3))
                            .with_default_span(),
                    ]
                ))
                .with_default_span()
            ),]))
        );
    }

    #[test]
    fn single_statement() {
        let src = "1;";
        let expr = parse_unwrap_data(src);
        assert_eq!(
            expr,
            DatexExpressionData::Statements(Statements::new_terminated(vec![
                DatexExpressionData::Integer(Integer::from(1))
                    .with_default_span()
            ]))
        );
    }

    #[test]
    fn empty_statement() {
        let src = ";";
        let expr = parse_unwrap_data(src);
        assert_eq!(expr, DatexExpressionData::Statements(Statements::empty()));
    }

    #[test]
    fn empty_statement_multiple() {
        let src = ";;;";
        let expr = parse_unwrap_data(src);
        assert_eq!(expr, DatexExpressionData::Statements(Statements::empty()));
    }

    #[test]
    fn variable_expression() {
        let src = "myVar";
        let expr = parse_unwrap_data(src);
        assert_eq!(expr, DatexExpressionData::Identifier("myVar".to_string()));
    }

    #[test]
    fn variable_expression_with_operations() {
        let src = "myVar + 1";
        let expr = parse_unwrap_data(src);
        assert_eq!(
            expr,
            DatexExpressionData::BinaryOperation(BinaryOperation {
                operator: BinaryOperator::Arithmetic(ArithmeticOperator::Add),
                left: Box::new(
                    DatexExpressionData::Identifier("myVar".to_string())
                        .with_default_span()
                ),
                right: Box::new(
                    DatexExpressionData::Integer(Integer::from(1))
                        .with_default_span()
                ),
                r#type: None
            })
        );
    }

    #[test]
    fn apply_expression() {
        let src = "myFunc(1, 2, 3)";
        let expr = parse_unwrap_data(src);
        assert_eq!(
            expr,
            DatexExpressionData::ApplyChain(ApplyChain {
                base: Box::new(
                    DatexExpressionData::Identifier("myFunc".to_string())
                        .with_default_span()
                ),
                operations: vec![ApplyOperation::FunctionCall(
                    DatexExpressionData::List(List::new(vec![
                        DatexExpressionData::Integer(Integer::from(1))
                            .with_default_span(),
                        DatexExpressionData::Integer(Integer::from(2))
                            .with_default_span(),
                        DatexExpressionData::Integer(Integer::from(3))
                            .with_default_span(),
                    ]))
                    .with_default_span()
                )],
            })
        );
    }

    #[test]
    fn apply_empty() {
        let src = "myFunc()";
        let expr = parse_unwrap_data(src);
        assert_eq!(
            expr,
            DatexExpressionData::ApplyChain(ApplyChain {
                base: Box::new(
                    DatexExpressionData::Identifier("myFunc".to_string())
                        .with_default_span()
                ),
                operations: vec![ApplyOperation::FunctionCall(
                    DatexExpressionData::Map(Map::new(vec![]))
                        .with_default_span()
                )],
            })
        );
    }

    #[test]
    fn apply_multiple() {
        let src = "myFunc(1)(2, 3)";
        let expr = parse_unwrap_data(src);
        assert_eq!(
            expr,
            DatexExpressionData::ApplyChain(ApplyChain {
                base: Box::new(
                    DatexExpressionData::Identifier("myFunc".to_string())
                        .with_default_span()
                ),
                operations: vec![
                    ApplyOperation::FunctionCall(
                        DatexExpressionData::List(List::new(vec![
                            DatexExpressionData::Integer(Integer::from(1))
                                .with_default_span()
                        ]))
                        .with_default_span()
                    ),
                    ApplyOperation::FunctionCall(
                        DatexExpressionData::List(List::new(vec![
                            DatexExpressionData::Integer(Integer::from(2))
                                .with_default_span(),
                            DatexExpressionData::Integer(Integer::from(3))
                                .with_default_span(),
                        ]))
                        .with_default_span()
                    )
                ],
            })
        );
    }

    #[test]
    fn apply_atom() {
        let src = "print 'test'";
        let expr = parse_unwrap_data(src);
        assert_eq!(
            expr,
            DatexExpressionData::ApplyChain(ApplyChain {
                base: Box::new(
                    DatexExpressionData::Identifier("print".to_string())
                        .with_default_span()
                ),
                operations: vec![ApplyOperation::FunctionCall(
                    DatexExpressionData::Text("test".to_string())
                        .with_default_span()
                )],
            })
        );
    }

    #[test]
    fn property_access() {
        let src = "myObj.myProp";
        let expr = parse_unwrap_data(src);
        assert_eq!(
            expr,
            DatexExpressionData::ApplyChain(ApplyChain {
                base: Box::new(
                    DatexExpressionData::Identifier("myObj".to_string())
                        .with_default_span()
                ),
                operations: vec![ApplyOperation::PropertyAccess(
                    DatexExpressionData::Text("myProp".to_string())
                        .with_default_span()
                )],
            })
        );
    }

    #[test]
    fn property_access_scoped() {
        let src = "myObj.(1)";
        let expr = parse_unwrap_data(src);
        assert_eq!(
            expr,
            DatexExpressionData::ApplyChain(ApplyChain {
                base: Box::new(
                    DatexExpressionData::Identifier("myObj".to_string())
                        .with_default_span()
                ),
                operations: vec![ApplyOperation::PropertyAccess(
                    DatexExpressionData::Integer(Integer::from(1))
                        .with_default_span()
                )],
            })
        );
    }

    #[test]
    fn property_access_multiple() {
        let src = "myObj.myProp.anotherProp.(1 + 2).(x;y)";
        let expr = parse_unwrap_data(src);
        assert_eq!(
            expr,
            DatexExpressionData::ApplyChain(ApplyChain {
                base: Box::new(
                    DatexExpressionData::Identifier("myObj".to_string())
                        .with_default_span()
                ),
                operations: vec![
                    ApplyOperation::PropertyAccess(
                        DatexExpressionData::Text("myProp".to_string())
                            .with_default_span()
                    ),
                    ApplyOperation::PropertyAccess(
                        DatexExpressionData::Text("anotherProp".to_string())
                            .with_default_span()
                    ),
                    ApplyOperation::PropertyAccess(
                        DatexExpressionData::BinaryOperation(BinaryOperation {
                            operator: BinaryOperator::Arithmetic(
                                ArithmeticOperator::Add
                            ),
                            left: Box::new(
                                DatexExpressionData::Integer(Integer::from(1))
                                    .with_default_span()
                            ),
                            right: Box::new(
                                DatexExpressionData::Integer(Integer::from(2))
                                    .with_default_span()
                            ),
                            r#type: None
                        })
                        .with_default_span()
                    ),
                    ApplyOperation::PropertyAccess(
                        DatexExpressionData::Statements(
                            Statements::new_unterminated(vec![
                                DatexExpressionData::Identifier(
                                    "x".to_string()
                                )
                                .with_default_span(),
                                DatexExpressionData::Identifier(
                                    "y".to_string()
                                )
                                .with_default_span(),
                            ])
                        )
                        .with_default_span()
                    ),
                ],
            })
        );
    }

    #[test]
    fn property_access_and_apply() {
        let src = "myObj.myProp(1, 2)";
        let expr = parse_unwrap_data(src);
        assert_eq!(
            expr,
            DatexExpressionData::ApplyChain(ApplyChain {
                base: Box::new(
                    DatexExpressionData::Identifier("myObj".to_string())
                        .with_default_span()
                ),
                operations: vec![
                    ApplyOperation::PropertyAccess(
                        DatexExpressionData::Text("myProp".to_string())
                            .with_default_span()
                    ),
                    ApplyOperation::FunctionCall(
                        DatexExpressionData::List(List::new(vec![
                            DatexExpressionData::Integer(Integer::from(1))
                                .with_default_span(),
                            DatexExpressionData::Integer(Integer::from(2))
                                .with_default_span(),
                        ]))
                        .with_default_span()
                    ),
                ]
            },)
        );
    }

    #[test]
    fn apply_and_property_access() {
        let src = "myFunc(1).myProp";
        let expr = parse_unwrap_data(src);
        assert_eq!(
            expr,
            DatexExpressionData::ApplyChain(ApplyChain {
                base: Box::new(
                    DatexExpressionData::Identifier("myFunc".to_string())
                        .with_default_span()
                ),
                operations: vec![
                    ApplyOperation::FunctionCall(
                        DatexExpressionData::List(List::new(vec![
                            DatexExpressionData::Integer(Integer::from(1))
                                .with_default_span(),
                        ]))
                        .with_default_span()
                    ),
                    ApplyOperation::PropertyAccess(
                        DatexExpressionData::Text("myProp".to_string())
                            .with_default_span()
                    ),
                ],
            })
        );
    }

    #[test]
    fn nested_apply_and_property_access() {
        let src = "((x(1)).y).z";
        let expr = parse_unwrap_data(src);
        assert_eq!(
            expr,
            DatexExpressionData::ApplyChain(ApplyChain {
                base: Box::new(
                    DatexExpressionData::ApplyChain(ApplyChain {
                        base: Box::new(
                            DatexExpressionData::ApplyChain(ApplyChain {
                                base: Box::new(
                                    DatexExpressionData::Identifier(
                                        "x".to_string()
                                    )
                                    .with_default_span()
                                ),
                                operations: vec![ApplyOperation::FunctionCall(
                                    DatexExpressionData::List(List::new(vec![
                                        DatexExpressionData::Integer(
                                            Integer::from(1)
                                        )
                                        .with_default_span()
                                    ]))
                                    .with_default_span()
                                )],
                            })
                            .with_default_span()
                        ),
                        operations: vec![ApplyOperation::PropertyAccess(
                            DatexExpressionData::Text("y".to_string())
                                .with_default_span()
                        )],
                    })
                    .with_default_span()
                ),
                operations: vec![ApplyOperation::PropertyAccess(
                    DatexExpressionData::Text("z".to_string())
                        .with_default_span()
                )],
            })
        );
    }

    #[test]
    fn type_declaration_statement() {
        let src = "type User = { age: 42, name: \"John\" };";
        let expr = parse_unwrap_data(src);
        assert_eq!(
            expr,
            DatexExpressionData::Statements(Statements::new_terminated(vec![
                DatexExpressionData::TypeDeclaration(TypeDeclaration {
                    id: None,
                    name: "User".to_string(),
                    value: TypeExpressionData::StructuralMap(StructuralMap(
                        vec![
                            (
                                TypeExpressionData::Text("age".to_string())
                                    .with_default_span(),
                                TypeExpressionData::Integer(Integer::from(42))
                                    .with_default_span()
                            ),
                            (
                                TypeExpressionData::Text("name".to_string())
                                    .with_default_span(),
                                TypeExpressionData::Text("John".to_string())
                                    .with_default_span()
                            ),
                        ]
                    ))
                    .with_default_span(),
                    hoisted: false,
                })
                .with_default_span()
            ]))
        );

        // make sure { type: 42, name: "John" } is not parsed as type declaration
        let src = r#"{ type: 42, name: "John" };"#;
        let expr = parse_unwrap_data(src);
        assert_eq!(
            expr,
            DatexExpressionData::Statements(Statements::new_terminated(vec![
                DatexExpressionData::Map(Map::new(vec![
                    (
                        DatexExpressionData::Text("type".to_string())
                            .with_default_span(),
                        DatexExpressionData::Integer(Integer::from(42))
                            .with_default_span()
                    ),
                    (
                        DatexExpressionData::Text("name".to_string())
                            .with_default_span(),
                        DatexExpressionData::Text("John".to_string())
                            .with_default_span()
                    ),
                ]))
                .with_default_span()
            ]))
        );
    }

    #[test]
    fn variable_declaration_statement() {
        let src = "const x = 42;";
        let expr = parse_unwrap_data(src);
        assert_eq!(
            expr,
            DatexExpressionData::Statements(Statements::new_terminated(vec![
                DatexExpressionData::VariableDeclaration(VariableDeclaration {
                    id: None,
                    kind: VariableKind::Const,
                    type_annotation: None,
                    name: "x".to_string(),
                    init_expression: Box::new(
                        DatexExpressionData::Integer(Integer::from(42))
                            .with_default_span()
                    ),
                })
                .with_default_span()
            ]))
        );
    }

    #[test]
    fn variable_declaration_with_expression() {
        let src = "var x = 1 + 2";
        let expr = parse_unwrap_data(src);
        assert_eq!(
            expr,
            DatexExpressionData::VariableDeclaration(VariableDeclaration {
                id: None,
                kind: VariableKind::Var,
                type_annotation: None,
                name: "x".to_string(),
                init_expression: Box::new(
                    DatexExpressionData::BinaryOperation(BinaryOperation {
                        operator: BinaryOperator::Arithmetic(
                            ArithmeticOperator::Add
                        ),
                        left: Box::new(
                            DatexExpressionData::Integer(Integer::from(1))
                                .with_default_span()
                        ),
                        right: Box::new(
                            DatexExpressionData::Integer(Integer::from(2))
                                .with_default_span()
                        ),
                        r#type: None
                    })
                    .with_default_span()
                )
            })
        );
    }

    #[test]
    fn variable_assignment() {
        let src = "x = 42";
        let expr = parse_unwrap_data(src);
        assert_eq!(
            expr,
            DatexExpressionData::VariableAssignment(VariableAssignment {
                id: None,
                operator: AssignmentOperator::Assign,
                name: "x".to_string(),
                expression: Box::new(
                    DatexExpressionData::Integer(Integer::from(42))
                        .with_default_span()
                ),
            })
        );
    }

    #[test]
    fn variable_assignment_expression() {
        let src = "x = (y = 1)";
        let expr = parse_unwrap_data(src);
        assert_eq!(
            expr,
            DatexExpressionData::VariableAssignment(VariableAssignment {
                id: None,
                operator: AssignmentOperator::Assign,
                name: "x".to_string(),
                expression: Box::new(
                    DatexExpressionData::VariableAssignment(
                        VariableAssignment {
                            id: None,
                            operator: AssignmentOperator::Assign,
                            name: "y".to_string(),
                            expression: Box::new(
                                DatexExpressionData::Integer(Integer::from(1))
                                    .with_default_span()
                            ),
                        }
                    )
                    .with_default_span()
                ),
            })
        );
    }

    #[test]
    fn variable_assignment_expression_in_list() {
        let src = "[x = 1]";
        let expr = parse_unwrap_data(src);
        assert_eq!(
            expr,
            DatexExpressionData::List(List::new(vec![
                DatexExpressionData::VariableAssignment(VariableAssignment {
                    id: None,
                    operator: AssignmentOperator::Assign,
                    name: "x".to_string(),
                    expression: Box::new(
                        DatexExpressionData::Integer(Integer::from(1))
                            .with_default_span()
                    ),
                })
                .with_default_span()
            ]))
        );
    }

    #[test]
    fn apply_in_list() {
        let src = "[myFunc(1)]";
        let expr = parse_unwrap_data(src);
        assert_eq!(
            expr,
            DatexExpressionData::List(List::new(vec![
                DatexExpressionData::ApplyChain(ApplyChain {
                    base: Box::new(
                        DatexExpressionData::Identifier("myFunc".to_string())
                            .with_default_span()
                    ),
                    operations: vec![ApplyOperation::FunctionCall(
                        DatexExpressionData::List(List::new(vec![
                            DatexExpressionData::Integer(Integer::from(1))
                                .with_default_span()
                        ]))
                        .with_default_span()
                    )]
                })
                .with_default_span()
            ]))
        );
    }

    #[test]
    fn variant_accessor() {
        let res = parse_unwrap_data("integer/u8");
        assert_eq!(
            res,
            DatexExpressionData::BinaryOperation(BinaryOperation {
                operator: BinaryOperator::VariantAccess,
                left: Box::new(
                    DatexExpressionData::Identifier("integer".to_string())
                        .with_default_span()
                ),
                right: Box::new(
                    DatexExpressionData::Identifier("u8".to_string())
                        .with_default_span()
                ),
                r#type: None
            })
        );

        let res = parse_unwrap_data("undeclared/u8");
        assert_eq!(
            res,
            DatexExpressionData::BinaryOperation(BinaryOperation {
                operator: BinaryOperator::VariantAccess,
                left: Box::new(
                    DatexExpressionData::Identifier("undeclared".to_string())
                        .with_default_span()
                ),
                right: Box::new(
                    DatexExpressionData::Identifier("u8".to_string())
                        .with_default_span()
                ),
                r#type: None
            })
        );
    }

    #[test]
    fn fraction() {
        // fraction
        let res = parse_unwrap_data("42/3");
        assert_eq!(
            res,
            DatexExpressionData::Decimal(Decimal::from_string("42/3").unwrap())
        );

        let src = "1/3";
        let val = parse_to_value_container(src);
        assert_eq!(
            val,
            ValueContainer::from(Decimal::from_string("1/3").unwrap())
        );

        // divison
        let res = parse_unwrap_data("42.4/3");
        assert_eq!(
            res,
            DatexExpressionData::BinaryOperation(BinaryOperation {
                operator: BinaryOperator::Arithmetic(
                    ArithmeticOperator::Divide
                ),
                left: Box::new(
                    DatexExpressionData::Decimal(
                        Decimal::from_string("42.4").unwrap()
                    )
                    .with_default_span()
                ),
                right: Box::new(
                    DatexExpressionData::Integer(Integer::from(3))
                        .with_default_span()
                ),
                r#type: None
            })
        );

        let res = parse_unwrap_data("42 /3");
        assert_eq!(
            res,
            DatexExpressionData::BinaryOperation(BinaryOperation {
                operator: BinaryOperator::Arithmetic(
                    ArithmeticOperator::Divide
                ),
                left: Box::new(
                    DatexExpressionData::Integer(Integer::from(42))
                        .with_default_span()
                ),
                right: Box::new(
                    DatexExpressionData::Integer(Integer::from(3))
                        .with_default_span()
                ),
                r#type: None
            })
        );

        let res = parse_unwrap_data("42/ 3");
        assert_eq!(
            res,
            DatexExpressionData::BinaryOperation(BinaryOperation {
                operator: BinaryOperator::Arithmetic(
                    ArithmeticOperator::Divide
                ),
                left: Box::new(
                    DatexExpressionData::Integer(Integer::from(42))
                        .with_default_span()
                ),
                right: Box::new(
                    DatexExpressionData::Integer(Integer::from(3))
                        .with_default_span()
                ),
                r#type: None
            })
        );
    }

    #[test]
    fn endpoint() {
        let src = "@jonas";
        let val = parse_to_value_container(src);
        assert_eq!(
            val,
            ValueContainer::from(Endpoint::from_str("@jonas").unwrap())
        );
    }

    // TODO #159:
    // #[test]
    // fn variable_assignment_multiple() {
    //     let src = "x = y = 42";
    //     let expr = parse_unwrap(src);
    //     assert_eq!(
    //         expr,
    //         DatexExpression::VariableAssignment(
    //             "x".to_string(),
    //             Box::new(DatexExpression::VariableAssignment(
    //                 "y".to_string(),
    //                 Box::new(DatexExpression::Integer(Integer::from(42))),
    //             )),
    //         )
    //     );
    // }

    #[test]
    fn variable_declaration_and_assignment() {
        let src = "var x = 42; x = 100 * 10;";
        let expr = parse_unwrap_data(src);
        assert_eq!(
            expr,
            DatexExpressionData::Statements(Statements::new_terminated(vec![
                DatexExpressionData::VariableDeclaration(VariableDeclaration {
                    id: None,
                    kind: VariableKind::Var,
                    name: "x".to_string(),
                    init_expression: Box::new(
                        DatexExpressionData::Integer(Integer::from(42))
                            .with_default_span()
                    ),
                    type_annotation: None
                })
                .with_default_span(),
                DatexExpressionData::VariableAssignment(VariableAssignment {
                    id: None,
                    operator: AssignmentOperator::Assign,
                    name: "x".to_string(),
                    expression: Box::new(
                        DatexExpressionData::BinaryOperation(BinaryOperation {
                            operator: BinaryOperator::Arithmetic(
                                ArithmeticOperator::Multiply
                            ),
                            left: Box::new(
                                DatexExpressionData::Integer(Integer::from(
                                    100
                                ))
                                .with_default_span()
                            ),
                            right: Box::new(
                                DatexExpressionData::Integer(Integer::from(10))
                                    .with_default_span()
                            ),
                            r#type: None
                        })
                        .with_default_span()
                    ),
                })
                .with_default_span()
            ]))
        );
    }

    #[test]
    fn placeholder() {
        let src = "?";
        let expr = parse_unwrap_data(src);
        assert_eq!(expr, DatexExpressionData::Placeholder);
    }

    #[test]
    fn integer_to_value_container() {
        let src = "123456789123456789";
        let val = parse_to_value_container(src);
        assert_eq!(
            val,
            ValueContainer::from(
                Integer::from_string("123456789123456789").unwrap()
            )
        );
    }

    #[test]
    fn decimal_to_value_container() {
        let src = "123.456789123456";
        let val = parse_to_value_container(src);
        assert_eq!(
            val,
            ValueContainer::from(
                Decimal::from_string("123.456789123456").unwrap()
            )
        );
    }

    #[test]
    fn text_to_value_container() {
        let src = r#""Hello, world!""#;
        let val = parse_to_value_container(src);
        assert_eq!(val, ValueContainer::from("Hello, world!".to_string()));
    }

    #[test]
    fn list_to_value_container() {
        let src = "[1, 2, 3, 4.5, \"text\"]";
        let val = parse_to_value_container(src);
        let value_container_list: Vec<ValueContainer> = vec![
            Integer::from(1).into(),
            Integer::from(2).into(),
            Integer::from(3).into(),
            Decimal::from_string("4.5").unwrap().into(),
            "text".to_string().into(),
        ];
        assert_eq!(val, ValueContainer::from(value_container_list));
    }

    #[test]
    fn json_to_value_container() {
        let src = r#"
            {
                "name": "Test",
                "value": 42,
                "active": true,
                "items": [1, 2, 3, 0.5],
                "nested": {
                    "key": "value"
                }
            }
        "#;

        let val = parse_to_value_container(src);
        let value_container_list: Vec<ValueContainer> = vec![
            Integer::from(1).into(),
            Integer::from(2).into(),
            Integer::from(3).into(),
            Decimal::from_string("0.5").unwrap().into(),
        ];
        let value_container_inner_map: ValueContainer =
            ValueContainer::from(crate::values::core_values::map::Map::from(
                vec![("key".to_string(), "value".to_string().into())]
                    .into_iter()
                    .collect::<HashMap<String, ValueContainer>>(),
            ));
        let value_container_map: ValueContainer =
            ValueContainer::from(crate::values::core_values::map::Map::from(
                vec![
                    ("name".to_string(), "Test".to_string().into()),
                    ("value".to_string(), Integer::from(42).into()),
                    ("active".to_string(), true.into()),
                    ("items".to_string(), value_container_list.into()),
                    ("nested".to_string(), value_container_inner_map),
                ]
                .into_iter()
                .collect::<HashMap<String, ValueContainer>>(),
            ));
        assert_eq!(val, value_container_map);
    }

    #[test]
    fn invalid_value_containers() {
        let src = "1 + 2";
        let expr = parse_unwrap_data(src);
        assert!(
            ValueContainer::try_from(&expr).is_err(),
            "Expected error when converting expression to ValueContainer"
        );

        let src = "xy";
        let expr = parse_unwrap_data(src);
        assert!(
            ValueContainer::try_from(&expr).is_err(),
            "Expected error when converting expression to ValueContainer"
        );

        let src = "x()";
        let expr = parse_unwrap_data(src);
        assert!(
            ValueContainer::try_from(&expr).is_err(),
            "Expected error when converting expression to ValueContainer"
        );
    }

    #[test]
    fn decimal_nan() {
        let src = "NaN";
        let num = parse_unwrap_data(src);
        assert_matches!(num, DatexExpressionData::Decimal(Decimal::NaN));

        let src = "nan";
        let num = parse_unwrap_data(src);
        assert_matches!(num, DatexExpressionData::Decimal(Decimal::NaN));
    }

    #[test]
    fn decimal_infinity() {
        let src = "Infinity";
        let num = parse_unwrap_data(src);
        assert_eq!(num, DatexExpressionData::Decimal(Decimal::Infinity));

        let src = "-Infinity";
        let num = parse_unwrap_data(src);
        assert_eq!(
            num,
            DatexExpressionData::UnaryOperation(UnaryOperation {
                operator: UnaryOperator::Arithmetic(
                    ArithmeticUnaryOperator::Minus
                ),
                expression: Box::new(
                    DatexExpressionData::Decimal(Decimal::Infinity)
                        .with_default_span()
                )
            })
        );

        let src = "infinity";
        let num = parse_unwrap_data(src);
        assert_eq!(num, DatexExpressionData::Decimal(Decimal::Infinity));

        let src = "-infinity";
        let num = parse_unwrap_data(src);

        assert_eq!(
            num,
            DatexExpressionData::UnaryOperation(UnaryOperation {
                operator: UnaryOperator::Arithmetic(
                    ArithmeticUnaryOperator::Minus
                ),
                expression: Box::new(
                    DatexExpressionData::Decimal(Decimal::Infinity)
                        .with_default_span()
                )
            })
        );
    }

    #[test]
    fn comment() {
        let src = "// This is a comment\n1 + 2";
        let expr = parse_unwrap_data(src);
        assert_eq!(
            expr,
            DatexExpressionData::BinaryOperation(BinaryOperation {
                operator: BinaryOperator::Arithmetic(ArithmeticOperator::Add),
                left: Box::new(
                    DatexExpressionData::Integer(Integer::from(1))
                        .with_default_span()
                ),
                right: Box::new(
                    DatexExpressionData::Integer(Integer::from(2))
                        .with_default_span()
                ),
                r#type: None
            })
        );

        let src = "1 + //test\n2";
        let expr = parse_unwrap_data(src);
        assert_eq!(
            expr,
            DatexExpressionData::BinaryOperation(BinaryOperation {
                operator: BinaryOperator::Arithmetic(ArithmeticOperator::Add),
                left: Box::new(
                    DatexExpressionData::Integer(Integer::from(1))
                        .with_default_span()
                ),
                right: Box::new(
                    DatexExpressionData::Integer(Integer::from(2))
                        .with_default_span()
                ),
                r#type: None
            })
        );
    }

    #[test]
    fn multiline_comment() {
        let src = "/* This is a\nmultiline comment */\n1 + 2";
        let expr = parse_unwrap_data(src);
        assert_eq!(
            expr,
            DatexExpressionData::BinaryOperation(BinaryOperation {
                operator: BinaryOperator::Arithmetic(ArithmeticOperator::Add),
                left: Box::new(
                    DatexExpressionData::Integer(Integer::from(1))
                        .with_default_span()
                ),
                right: Box::new(
                    DatexExpressionData::Integer(Integer::from(2))
                        .with_default_span()
                ),
                r#type: None
            })
        );

        let src = "1 + /*test*/ 2";
        let expr = parse_unwrap_data(src);
        assert_eq!(
            expr,
            DatexExpressionData::BinaryOperation(BinaryOperation {
                operator: BinaryOperator::Arithmetic(ArithmeticOperator::Add),
                left: Box::new(
                    DatexExpressionData::Integer(Integer::from(1))
                        .with_default_span()
                ),
                right: Box::new(
                    DatexExpressionData::Integer(Integer::from(2))
                        .with_default_span()
                ),
                r#type: None
            })
        );
    }

    #[test]
    fn shebang() {
        let src = "#!/usr/bin/env datex\n1 + 2";
        let expr = parse_unwrap_data(src);
        assert_eq!(
            expr,
            DatexExpressionData::BinaryOperation(BinaryOperation {
                operator: BinaryOperator::Arithmetic(ArithmeticOperator::Add),
                left: Box::new(
                    DatexExpressionData::Integer(Integer::from(1))
                        .with_default_span()
                ),
                right: Box::new(
                    DatexExpressionData::Integer(Integer::from(2))
                        .with_default_span()
                ),
                r#type: None
            })
        );

        let src = "1;\n#!/usr/bin/env datex\n2";
        // syntax error
        let res = parse(src);
        assert!(
            !res.is_valid(),
            "Expected error when parsing expression with shebang"
        );
    }

    #[test]
    fn remote_execution() {
        let src = "a :: b";
        let expr = parse_unwrap_data(src);
        assert_eq!(
            expr,
            DatexExpressionData::RemoteExecution(RemoteExecution {
                left: Box::new(
                    DatexExpressionData::Identifier("a".to_string())
                        .with_default_span()
                ),
                right: Box::new(
                    DatexExpressionData::Identifier("b".to_string())
                        .with_default_span()
                )
            })
        );
    }
    #[test]
    fn remote_execution_no_space() {
        let src = "a::b";
        let expr = parse_unwrap_data(src);
        assert_eq!(
            expr,
            DatexExpressionData::RemoteExecution(RemoteExecution {
                left: Box::new(
                    DatexExpressionData::Identifier("a".to_string())
                        .with_default_span()
                ),
                right: Box::new(
                    DatexExpressionData::Identifier("b".to_string())
                        .with_default_span()
                )
            })
        );
    }

    #[test]
    fn remote_execution_complex() {
        let src = "a :: b + c * 2";
        let expr = parse_unwrap_data(src);
        assert_eq!(
            expr,
            DatexExpressionData::RemoteExecution(RemoteExecution {
                left: Box::new(
                    DatexExpressionData::Identifier("a".to_string())
                        .with_default_span()
                ),
                right: Box::new(
                    DatexExpressionData::BinaryOperation(BinaryOperation {
                        operator: BinaryOperator::Arithmetic(
                            ArithmeticOperator::Add
                        ),
                        left: Box::new(
                            DatexExpressionData::Identifier("b".to_string())
                                .with_default_span()
                        ),
                        right: Box::new(
                            DatexExpressionData::BinaryOperation(
                                BinaryOperation {
                                    operator: BinaryOperator::Arithmetic(
                                        ArithmeticOperator::Multiply
                                    ),
                                    left: Box::new(
                                        DatexExpressionData::Identifier(
                                            "c".to_string()
                                        )
                                        .with_default_span()
                                    ),
                                    right: Box::new(
                                        DatexExpressionData::Integer(
                                            Integer::from(2)
                                        )
                                        .with_default_span()
                                    ),
                                    r#type: None
                                }
                            )
                            .with_default_span()
                        ),
                        r#type: None
                    })
                    .with_default_span()
                )
            },)
        );
    }

    #[test]
    fn remote_execution_statements() {
        let src = "a :: b; 1";
        let expr = parse_unwrap_data(src);
        assert_eq!(
            expr,
            DatexExpressionData::Statements(Statements::new_unterminated(
                vec![
                    DatexExpressionData::RemoteExecution(RemoteExecution {
                        left: Box::new(
                            DatexExpressionData::Identifier("a".to_string())
                                .with_default_span()
                        ),
                        right: Box::new(
                            DatexExpressionData::Identifier("b".to_string())
                                .with_default_span()
                        )
                    })
                    .with_default_span(),
                    DatexExpressionData::Integer(Integer::from(1))
                        .with_default_span(),
                ]
            ))
        );
    }

    #[test]
    fn remote_execution_inline_statements() {
        let src = "a :: (1; 2 + 3)";
        let expr = parse_unwrap_data(src);
        assert_eq!(
            expr,
            DatexExpressionData::RemoteExecution(RemoteExecution {
                left: Box::new(
                    DatexExpressionData::Identifier("a".to_string())
                        .with_default_span()
                ),
                right: Box::new(
                    DatexExpressionData::Statements(
                        Statements::new_unterminated(vec![
                            DatexExpressionData::Integer(Integer::from(1))
                                .with_default_span(),
                            DatexExpressionData::BinaryOperation(
                                BinaryOperation {
                                    operator: BinaryOperator::Arithmetic(
                                        ArithmeticOperator::Add
                                    ),
                                    left: Box::new(
                                        DatexExpressionData::Integer(
                                            Integer::from(2)
                                        )
                                        .with_default_span()
                                    ),
                                    right: Box::new(
                                        DatexExpressionData::Integer(
                                            Integer::from(3)
                                        )
                                        .with_default_span()
                                    ),
                                    r#type: None
                                }
                            )
                            .with_default_span(),
                        ])
                    )
                    .with_default_span()
                )
            },)
        );
    }

    #[test]
    fn named_slot() {
        let src = "#endpoint";
        let expr = parse_unwrap_data(src);
        assert_eq!(
            expr,
            DatexExpressionData::Slot(Slot::Named("endpoint".to_string()))
        );
    }

    #[test]
    fn deref() {
        let src = "*x";
        let expr = parse_unwrap_data(src);
        assert_eq!(
            expr,
            DatexExpressionData::Deref(Box::new(
                DatexExpressionData::Identifier("x".to_string())
                    .with_default_span()
            ))
        );
    }

    #[test]
    fn deref_multiple() {
        let src = "**x";
        let expr = parse_unwrap_data(src);
        assert_eq!(
            expr,
            DatexExpressionData::Deref(Box::new(
                DatexExpressionData::Deref(Box::new(
                    DatexExpressionData::Identifier("x".to_string())
                        .with_default_span()
                ))
                .with_default_span()
            ))
        );
    }

    #[test]
    fn addressed_slot() {
        let src = "#123";
        let expr = parse_unwrap_data(src);
        assert_eq!(expr, DatexExpressionData::Slot(Slot::Addressed(123)));
    }

    #[test]
    fn pointer_address() {
        // 3 bytes (internal)
        let src = "$123456";
        let expr = parse_unwrap_data(src);
        assert_eq!(
            expr,
            DatexExpressionData::PointerAddress(PointerAddress::Internal([
                0x12, 0x34, 0x56
            ]))
        );

        // 5 bytes (local)
        let src = "$123456789A";
        let expr = parse_unwrap_data(src);
        assert_eq!(
            expr,
            DatexExpressionData::PointerAddress(PointerAddress::Local([
                0x12, 0x34, 0x56, 0x78, 0x9A
            ]))
        );

        // 26 bytes (remote)
        let src = "$1234567890ABCDEF123456789000000000000000000000000042";
        let expr = parse_unwrap_data(src);
        assert_eq!(
            expr,
            DatexExpressionData::PointerAddress(PointerAddress::Remote([
                0x12, 0x34, 0x56, 0x78, 0x90, 0xAB, 0xCD, 0xEF, 0x12, 0x34,
                0x56, 0x78, 0x90, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x42
            ]))
        );

        // other lengths are invalid
        let src = "$12";
        let res = parse(src);
        assert!(!res.is_valid());
    }

    #[test]
    fn variable_add_assignment() {
        let src = "x += 42";
        let expr = parse_unwrap_data(src);
        assert_eq!(
            expr,
            DatexExpressionData::VariableAssignment(VariableAssignment {
                id: None,
                operator: AssignmentOperator::AddAssign,
                name: "x".to_string(),
                expression: Box::new(
                    DatexExpressionData::Integer(Integer::from(42))
                        .with_default_span()
                ),
            })
        );
    }

    #[test]
    fn variable_sub_assignment() {
        let src = "x -= 42";
        let expr = parse_unwrap_data(src);
        assert_eq!(
            expr,
            DatexExpressionData::VariableAssignment(VariableAssignment {
                id: None,
                operator: AssignmentOperator::SubtractAssign,
                name: "x".to_string(),
                expression: Box::new(
                    DatexExpressionData::Integer(Integer::from(42))
                        .with_default_span()
                ),
            })
        );
    }

    #[test]
    fn variable_declaration_mut() {
        let src = "const x = &mut [1, 2, 3]";
        let expr = parse_unwrap_data(src);
        assert_eq!(
            expr,
            DatexExpressionData::VariableDeclaration(VariableDeclaration {
                id: None,
                kind: VariableKind::Const,
                name: "x".to_string(),
                type_annotation: None,
                init_expression: Box::new(
                    DatexExpressionData::CreateRefMut(Box::new(
                        DatexExpressionData::List(List::new(vec![
                            DatexExpressionData::Integer(Integer::from(1))
                                .with_default_span(),
                            DatexExpressionData::Integer(Integer::from(2))
                                .with_default_span(),
                            DatexExpressionData::Integer(Integer::from(3))
                                .with_default_span(),
                        ]))
                        .with_default_span()
                    ))
                    .with_default_span()
                ),
            })
        );
    }

    #[test]
    fn variable_declaration_ref() {
        let src = "const x = &[1, 2, 3]";
        let expr = parse_unwrap_data(src);
        assert_eq!(
            expr,
            DatexExpressionData::VariableDeclaration(VariableDeclaration {
                id: None,
                kind: VariableKind::Const,
                name: "x".to_string(),
                type_annotation: None,
                init_expression: Box::new(
                    DatexExpressionData::CreateRef(Box::new(
                        DatexExpressionData::List(List::new(vec![
                            DatexExpressionData::Integer(Integer::from(1))
                                .with_default_span(),
                            DatexExpressionData::Integer(Integer::from(2))
                                .with_default_span(),
                            DatexExpressionData::Integer(Integer::from(3))
                                .with_default_span(),
                        ]))
                        .with_default_span()
                    ))
                    .with_default_span()
                ),
            })
        );
    }
    #[test]
    fn variable_declaration() {
        let src = "const x = 1";
        let expr = parse_unwrap_data(src);
        assert_eq!(
            expr,
            DatexExpressionData::VariableDeclaration(VariableDeclaration {
                id: None,
                kind: VariableKind::Const,
                name: "x".to_string(),
                type_annotation: None,
                init_expression: Box::new(
                    DatexExpressionData::Integer(Integer::from(1))
                        .with_default_span()
                ),
            })
        );
    }

    #[test]
    fn negation() {
        let src = "!x";
        let expr = parse_unwrap_data(src);
        assert_eq!(
            expr,
            DatexExpressionData::UnaryOperation(UnaryOperation {
                operator: UnaryOperator::Logical(LogicalUnaryOperator::Not),
                expression: Box::new(
                    DatexExpressionData::Identifier("x".to_string())
                        .with_default_span()
                )
            })
        );

        let src = "!true";
        let expr = parse_unwrap_data(src);
        assert_eq!(
            expr,
            DatexExpressionData::UnaryOperation(UnaryOperation {
                operator: UnaryOperator::Logical(LogicalUnaryOperator::Not),
                expression: Box::new(
                    DatexExpressionData::Boolean(true).with_default_span()
                )
            })
        );

        let src = "!![1, 2]";
        let expr = parse_unwrap_data(src);
        assert_matches!(
            expr,
            DatexExpressionData::UnaryOperation(UnaryOperation {
                operator: UnaryOperator::Logical(LogicalUnaryOperator::Not),
                expression:
                    box DatexExpression {
                        data:
                            DatexExpressionData::UnaryOperation(UnaryOperation {
                                operator:
                                    UnaryOperator::Logical(
                                        LogicalUnaryOperator::Not,
                                    ),
                                expression:
                                    box DatexExpression {
                                        data: DatexExpressionData::List(_),
                                        ..
                                    },
                            }),
                        ..
                    },
            })
        );
    }

    #[test]
    fn token_spans() {
        let src = "'test'+'x'";
        let expr = parse_unwrap(src);
        println!("Expr: {:#?}", expr);
        assert_eq!(expr.span.start, 0);
        assert_eq!(expr.span.end, 3);
        if let DatexExpressionData::BinaryOperation(BinaryOperation {
            operator: _,
            left,
            right,
            ..
        }) = expr.data
        {
            assert_eq!(left.span.start, 0);
            assert_eq!(left.span.end, 1);
            assert_eq!(right.span.start, 2);
            assert_eq!(right.span.end, 3);
        } else {
            panic!("Expected BinaryOperation");
        }
    }
}
