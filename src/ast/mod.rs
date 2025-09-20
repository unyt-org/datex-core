pub mod array;
pub mod assignment_operation;
pub mod atom;
pub mod binary_operation;
pub mod binding;
pub mod chain;
pub mod comparison_operation;
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
pub mod structure;
pub mod text;
pub mod r#type;
pub mod unary;
pub mod unary_operation;
pub mod utils;

use crate::ast::array::*;
use crate::ast::assignment_operation::*;
use crate::ast::atom::*;
use crate::ast::binary_operation::*;
use crate::ast::binding::*;
use crate::ast::chain::*;
use crate::ast::comparison_operation::*;
use crate::ast::error::error::ParseError;
use crate::ast::error::pattern::Pattern;
use crate::ast::function::*;
use crate::ast::key::*;
use crate::ast::map::*;
use crate::ast::structure::*;
use crate::ast::r#type::type_expression;
use crate::ast::unary::*;
use crate::ast::unary_operation::*;
use crate::ast::utils::*;

use crate::ast::list::list;
use crate::values::core_values::array::Array;
use crate::values::core_values::decimal::decimal::Decimal;
use crate::values::core_values::decimal::typed_decimal::TypedDecimal;
use crate::values::core_values::endpoint::Endpoint;
use crate::values::core_values::integer::integer::Integer;
use crate::values::core_values::integer::typed_integer::TypedInteger;
use crate::values::core_values::list::List;
use crate::values::core_values::map::Map;
use crate::values::core_values::r#type::Type;
use crate::values::pointer::PointerAddress;
use crate::values::reference::ReferenceMutability;
use crate::values::value::Value;
use crate::values::value_container::ValueContainer;
use chumsky::extra::Err;
use chumsky::prelude::*;
use lexer::Token;
use logos::Logos;
use std::{collections::HashMap, ops::Range};

pub type TokenInput<'a, X = Token> = &'a [X];
pub trait DatexParserTrait<'a, T = DatexExpression, X = Token> =
    Parser<'a, TokenInput<'a, Token>, T, Err<ParseError>> + Clone + 'a
    where X: PartialEq + 'a;

pub type DatexScriptParser<'a> =
    Boxed<'a, 'a, TokenInput<'a>, DatexExpression, Err<ParseError>>;

#[derive(Clone, Debug, PartialEq)]
pub struct Statement {
    pub expression: DatexExpression,
    pub is_terminated: bool,
}
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
                        DatexExpression::Recover
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

// TODO TBD can we deprecate this?
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum VariableKind {
    Const,
    Var,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BindingMutability {
    Immutable, // e.g. `const x = ...`
    Mutable,   // e.g. `var x = ...`
}

// #[deprecated(note = "Use other ReferenceMutability instead")]
// #[derive(Clone, Copy, Debug, PartialEq)]
// pub enum ReferenceMutability {
//     Mutable,
//     Immutable,
//     None,
// }

#[derive(Clone, Debug, PartialEq)]
pub enum Slot {
    Addressed(u32),
    Named(String),
}

// TODO: parse TypeExpressions in ast parser
#[derive(Clone, Debug, PartialEq)]
pub enum TypeExpression {
    Null,
    // a type name or variable, e.g. integer, string, User, MyType, T
    Literal(String),

    // literals
    Integer(Integer),
    TypedInteger(TypedInteger),
    Decimal(Decimal),
    TypedDecimal(TypedDecimal),
    Boolean(bool),
    Text(String),
    Endpoint(Endpoint),

    // [integer, text, endpoint]
    // size known to compile time, arbitrary types
    Array(Vec<TypeExpression>),

    // [text; 3], integer[10]
    // fixed size and known to compile time, only one type
    FixedSizeArray(Box<TypeExpression>, usize),

    // text[], integer[]
    // size not known to compile time, only one type
    SliceArray(Box<TypeExpression>),

    // { x: integer, y: text }
    Struct(Vec<(String, TypeExpression)>),

    // text & "test"
    Intersection(Vec<TypeExpression>),

    // text | integer
    Union(Vec<TypeExpression>),

    // User<text, integer>
    Generic(String, Vec<TypeExpression>),

    // (x: text) -> text
    Function {
        parameters: Vec<(String, TypeExpression)>,
        return_type: Box<TypeExpression>,
    },

    // List<integer>
    List(Box<TypeExpression>),

    // Map<integer, text>
    Map(Box<TypeExpression>, Box<TypeExpression>),

    // modifiers
    Ref(Box<TypeExpression>),
    RefMut(Box<TypeExpression>),
    RefFinal(Box<TypeExpression>),
}

#[derive(Clone, Debug, PartialEq)]
pub enum DatexExpression {
    /// This is a marker for recovery from parse errors.
    /// We should never use this manually.
    Recover,

    /// null
    Null,
    /// Boolean (true or false)
    Boolean(bool),
    /// Text, e.g "Hello, world!"
    Text(String),
    /// Decimal, e.g 123.456789123456
    Decimal(Decimal),

    TypedDecimal(TypedDecimal),

    /// Integer, e.g 123456789123456789
    Integer(Integer),

    /// Typed Integer, e.g. 123i8
    TypedInteger(TypedInteger),

    // Literal type, e.g. string, User or integer/u8
    Literal(String),

    /// Endpoint, e.g. @test_a or @test_b
    Endpoint(Endpoint),
    /// Array, e.g  `[1, 2, 3, "text"]`
    Array(Vec<DatexExpression>),
    /// List, e.g  `(1, 2, 3, "text")`, or without brackets: 1, 2, 3, "text"
    List(Vec<DatexExpression>),
    /// Struct, e.g {"key": "value", key2: 2, xy: 10}
    Struct(Vec<(String, DatexExpression)>),
    /// Map, e.g (1: 2, 3: 4, xy: "xy") or without brackets: a:3, b:4
    Map(Vec<(DatexExpression, DatexExpression)>),
    /// One or more statements, e.g (1; 2; 3)
    Statements(Vec<Statement>),
    /// Identifier, e.g. a variable name. VariableId is always set to 0 by the ast parser.
    Variable(Option<VariableId>, String),
    /// reference access, e.g. &<ABCDEF>
    GetReference(PointerAddress),

    /// Conditional expression, e.g. if (true) { 1 } else { 2 }
    Conditional {
        condition: Box<DatexExpression>,
        then_branch: Box<DatexExpression>,
        else_branch: Option<Box<DatexExpression>>,
    },

    /// Variable declaration, e.g. const x = 1, const mut x = 1, or var y = 2. VariableId is always set to 0 by the ast parser.
    VariableDeclaration {
        id: Option<VariableId>,
        kind: VariableKind,
        binding_mutability: BindingMutability,
        name: String,
        type_annotation: Option<TypeExpression>,
        value: Box<DatexExpression>,
    },

    /// Type declaration, e.g. type MyType = { x: 42, y: "John" };
    TypeDeclaration {
        id: Option<VariableId>,
        name: String,
        value: TypeExpression, // Type
    },

    /// Type expression, e.g. { x: 42, y: "John" }
    TypeExpression(TypeExpression),

    /// Type keyword, e.g. type(...)
    Type(TypeExpression),

    FunctionDeclaration {
        name: String,
        parameters: Box<DatexExpression>,
        return_type: Option<TypeExpression>,
        body: Box<DatexExpression>,
    },

    /// Reference, e.g. &x
    Ref(Box<DatexExpression>),
    /// Mutable reference, e.g. &mut x
    RefMut(Box<DatexExpression>),
    /// Final reference, e.g. &final x
    RefFinal(Box<DatexExpression>),

    /// Slot, e.g. #1, #endpoint
    Slot(Slot),
    /// Slot assignment
    SlotAssignment(Slot, Box<DatexExpression>),

    BinaryOperation(
        BinaryOperator,
        Box<DatexExpression>,
        Box<DatexExpression>,
        Option<Type>,
    ),
    ComparisonOperation(
        ComparisonOperator,
        Box<DatexExpression>,
        Box<DatexExpression>,
    ),
    AssignmentOperation(
        AssignmentOperator,
        Option<VariableId>,
        String,
        Box<DatexExpression>,
    ),
    UnaryOperation(UnaryOperator, Box<DatexExpression>),

    // apply (e.g. x (1)) or property access
    ApplyChain(Box<DatexExpression>, Vec<ApplyOperation>),

    // ?
    Placeholder,
    // @xy :: z
    RemoteExecution(Box<DatexExpression>, Box<DatexExpression>),
}

// directly convert DatexExpression to a ValueContainer
impl TryFrom<&DatexExpression> for ValueContainer {
    type Error = ();

    fn try_from(expr: &DatexExpression) -> Result<Self, Self::Error> {
        Ok(match expr {
            DatexExpression::Null => ValueContainer::Value(Value::null()),
            DatexExpression::Boolean(b) => ValueContainer::from(*b),
            DatexExpression::Text(s) => ValueContainer::from(s.clone()),
            DatexExpression::Decimal(d) => ValueContainer::from(d.clone()),
            DatexExpression::Integer(i) => ValueContainer::from(i.clone()),
            DatexExpression::Endpoint(e) => ValueContainer::from(e.clone()),
            DatexExpression::List(arr) => {
                let entries = arr
                    .iter()
                    .map(ValueContainer::try_from)
                    .collect::<Result<Vec<ValueContainer>, ()>>()?;
                ValueContainer::from(List::from(entries))
            }
            DatexExpression::Struct(obj) => {
                let entries = obj
                    .iter()
                    .map(|(k, v)| {
                        let value = ValueContainer::try_from(v)?;
                        Ok((k.clone(), value))
                    })
                    .collect::<Result<HashMap<String, ValueContainer>, ()>>()?;
                ValueContainer::from(Map::from(entries))
            }
            DatexExpression::Array(arr) => {
                let entries = arr
                    .iter()
                    .map(ValueContainer::try_from)
                    .collect::<Result<Vec<ValueContainer>, ()>>()?;
                ValueContainer::from(Array::from(entries))
            }
            DatexExpression::Map(pairs) => {
                let entries = pairs
                    .iter()
                    .map(|(k, v)| {
                        let key = ValueContainer::try_from(k)?;
                        let value = ValueContainer::try_from(v)?;
                        Ok((key, value))
                    })
                    .collect::<Result<HashMap<ValueContainer, ValueContainer>, ()>>()?;
                ValueContainer::from(Map::from(entries))
            }
            _ => Err(())?,
        })
    }
}

pub struct DatexParseResult {
    pub expression: DatexExpression,
    pub is_static_value: bool,
}

pub fn create_parser<'a, T>()
-> impl DatexParserTrait<'a, DatexExpression, Token>
where
    T: std::cmp::PartialEq + 'a,
{
    // an expression
    let mut expression = Recursive::declare();
    let mut expression_without_list = Recursive::declare();

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
        .map(|(exprs, last)| {
            // Convert expressions with mandatory semicolon
            let mut statements: Vec<Statement> = exprs
                .into_iter()
                .map(|expr| Statement {
                    expression: expr,
                    is_terminated: true,
                })
                .collect();

            if let Some((last_expr, last_semi)) = last {
                // If there's a last expression, add it as a statement
                statements.push(Statement {
                    expression: last_expr,
                    is_terminated: last_semi.is_some(),
                });
            }
            // if single statement without semicolon, treat it as a single expression
            if statements.len() == 1 && !statements[0].is_terminated {
                statements.remove(0).expression
            } else {
                DatexExpression::Statements(statements)
            }
        })
        .boxed()
        .labelled(Pattern::Custom("statements"));

    // expression wrapped in parentheses
    let wrapped_expression = statements
        .clone()
        .delimited_by(just(Token::LeftParen), just(Token::RightParen));
    //.labelled(Pattern::Custom("wrapped"))
    //.as_context();

    // a valid object/list key
    // (1: value), "key", 1, (("x"+"y"): 123)
    let key = key(wrapped_expression.clone()).labelled(Pattern::Custom("key"));

    // array
    // 1,2,3
    // [1,2,3,4,13434,(1),4,5,7,8]
    let array = array(expression_without_list.clone());

    let list = list(expression_without_list.clone());

    // object
    let structure = structure(expression_without_list.clone());

    // map
    // Key-value pair
    let map = map(key.clone(), expression_without_list.clone());

    // atomic expression (e.g. 1, "text", (1 + 2), (1;2))
    let atom =
        atom(array.clone(), structure.clone(), wrapped_expression.clone());
    let unary = choice((type_expression(), unary(atom.clone())));

    // apply chain: two expressions following each other directly, optionally separated with "." (property access)
    let chain = chain(
        unary.clone(),
        key.clone(),
        array.clone(),
        structure.clone(),
        wrapped_expression.clone(),
        atom.clone(),
        expression.clone(),
    );

    let reference = just(Token::Ampersand)
        .ignore_then(
            just(Token::Mutable)
                .or(just(Token::Final))
                .or_not()
                .padded_by(whitespace()),
        )
        .then(chain.clone())
        .map(|(ref_type, expr)| match ref_type {
            Some(Token::Mutable) => DatexExpression::RefMut(Box::new(expr)),
            Some(Token::Final) => DatexExpression::RefFinal(Box::new(expr)),
            None => DatexExpression::Ref(Box::new(expr)),
            _ => unreachable!(),
        });

    let unary = reference.clone().or(unary);
    let reference_or_chain = reference.or(chain.clone());

    let union = binary_operation(reference_or_chain);

    // FIXME WIP
    let function_declaration = function(statements.clone(), map.clone());

    // comparison (==, !=, is, …)
    let comparison = comparison_operation(union.clone());

    // declarations or assignments
    let declaration_or_assignment = declaration_or_assignment(union);

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
                    array.clone(),
                    structure.clone(),
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
                        array.clone(),
                        structure.clone(),
                        statements.clone(),
                        unary.clone(),
                    )))
                    .or_not(),
            )
            .map(|((cond, then_branch), else_opt)| {
                DatexExpression::Conditional {
                    condition: Box::new(cond),
                    then_branch: Box::new(unwrap_single_statement(then_branch)),
                    else_branch: else_opt
                        .map(unwrap_single_statement)
                        .map(Box::new),
                }
            })
            .boxed()
    });

    expression_without_list.define(choice((
        if_expression,
        declaration_or_assignment,
        function_declaration,
        comparison,
    )));

    // expression :: expression
    let remote_execution = expression_without_list
        .clone()
        .then_ignore(just(Token::DoubleColon).padded_by(whitespace()))
        .then(expression_without_list.clone())
        .map(|(endpoint, expr)| {
            DatexExpression::RemoteExecution(Box::new(endpoint), Box::new(expr))
        });

    expression.define(
        choice((
            remote_execution,
            list.clone(),
            map.clone(),
            expression_without_list.clone(),
        ))
        .padded_by(whitespace()),
    );

    choice((
        // empty script (0-n semicolons)
        just(Token::Semicolon)
            .repeated()
            .at_least(1)
            .padded_by(whitespace())
            .map(|_| DatexExpression::Statements(vec![])),
        // statements
        statements,
    ))
}

pub fn parse(mut src: &str) -> Result<DatexExpression, Vec<ParseError>> {
    // strip shebang at beginning of the source code
    if src.starts_with("#!") {
        let end_of_line = src.find('\n').unwrap_or(src.len());
        src = &src[end_of_line + 1..];
    }

    let tokens = Token::lexer(src);
    let tokens_spanned: Vec<(Token, Range<usize>)> = tokens
        .spanned()
        .map(|(tok, span)| {
            tok.map(|t| (t, span.clone()))
                .map_err(|_| ParseError::new_unexpected_with_span(None, span))
        })
        .collect::<Result<_, _>>()
        .map_err(|e| vec![e])?;

    let (tokens, spans): (Vec<_>, Vec<_>) = tokens_spanned.into_iter().unzip();
    let parser = create_parser::<'_, Token>();
    parser.parse(&tokens).into_result().map_err(|err| {
        err.into_iter()
            .map(|e| {
                let mut owned_error: ParseError = e.clone();
                let mut index = owned_error.token_pos().unwrap();
                if index >= spans.len() {
                    // FIXME how to show file end?
                    index = spans.len() - 1;
                }
                let span = spans.get(index).unwrap();
                owned_error.set_span(span.clone());
                owned_error
            })
            .collect()
    })
}

#[cfg(test)]
mod tests {
    use crate::{
        ast::error::{error::ErrorKind, pattern::Pattern, src::SrcId},
        values::core_values::endpoint::InvalidEndpointError,
    };

    use super::*;
    use std::{assert_matches::assert_matches, io, str::FromStr, vec};

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

    fn parse_print_error(
        src: &str,
    ) -> Result<DatexExpression, Vec<ParseError>> {
        let src_id = SrcId::test();
        let res = parse(src);
        if let Err(errors) = &res {
            errors.iter().for_each(|e| {
                let cache = ariadne::sources(vec![(src_id, src)]);
                e.clone().write(cache, io::stdout());
            });
        }
        res
    }

    fn try_parse_to_value_container(src: &str) -> ValueContainer {
        let expr = parse_unwrap(src);
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

        let json = parse_unwrap(src);

        assert_eq!(
            json,
            DatexExpression::Struct(vec![
                (
                    "name".to_string(),
                    DatexExpression::Text("Test".to_string())
                ),
                (
                    "value".to_string(),
                    DatexExpression::Integer(Integer::from(42))
                ),
                ("active".to_string(), DatexExpression::Boolean(true)),
                (
                    "items".to_string(),
                    DatexExpression::Array(vec![
                        DatexExpression::Integer(Integer::from(1)),
                        DatexExpression::Integer(Integer::from(2)),
                        DatexExpression::Integer(Integer::from(3)),
                        DatexExpression::Decimal(
                            Decimal::from_string("0.5").unwrap()
                        )
                    ])
                ),
                (
                    "nested".to_string(),
                    DatexExpression::Struct(
                        vec![(
                            "key".to_string(),
                            DatexExpression::Text("value".to_string())
                        )]
                        .into_iter()
                        .collect()
                    )
                ),
            ])
        );
    }

    #[test]
    #[ignore = "WIP"]
    fn type_expression() {
        let src = "type(1 | 2)";
        let result = parse_print_error(src);
        let expr = result.unwrap();
        assert_matches!(expr, DatexExpression::Type(TypeExpression::Union(_)));

        let src = "var a = type(1,2,3)";
        let result = parse_print_error(src);
        let expr = result.unwrap();
        if let DatexExpression::VariableDeclaration { value, .. } = expr {
            assert_matches!(
                *value,
                DatexExpression::Type(TypeExpression::List(_))
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
        assert_matches!(expr, DatexExpression::TypeDeclaration { name, .. } if name == "A");
    }

    #[test]
    fn nominal_type_declaration() {
        let src = "type B = { x: integer, y: string }";
        let result = parse_print_error(src);
        let expr = result.unwrap();
        assert_matches!(expr, DatexExpression::TypeDeclaration { name, .. } if name == "B");

        let src = "type User<T> = {id: T}";
        let result = parse_print_error(src);
        let expr = result.unwrap();
        assert_matches!(expr, DatexExpression::TypeDeclaration { name, .. } if name == "User");
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
        let val = parse_unwrap(src);
        assert_eq!(
            val,
            DatexExpression::FunctionDeclaration {
                name: "myFunction".to_string(),
                parameters: Box::new(DatexExpression::Map(vec![])),
                return_type: None,
                body: Box::new(DatexExpression::Integer(Integer::from(42))),
            }
        );
    }

    #[test]
    fn function_with_params() {
        let src = r#"
            function myFunction(x: integer) (
                42
            )
        "#;
        let val = parse_unwrap(src);
        assert_eq!(
            val,
            DatexExpression::FunctionDeclaration {
                name: "myFunction".to_string(),
                parameters: Box::new(DatexExpression::Map(vec![(
                    DatexExpression::Text("x".to_string()),
                    DatexExpression::Literal("integer".to_owned())
                )])),
                return_type: None,
                body: Box::new(DatexExpression::Integer(Integer::from(42))),
            }
        );

        let src = r#"
            function myFunction(x: integer, y: integer) (
                1 + 2;
            )
        "#;
        let val = parse_unwrap(src);
        assert_eq!(
            val,
            DatexExpression::FunctionDeclaration {
                name: "myFunction".to_string(),
                parameters: Box::new(DatexExpression::Map(vec![
                    (
                        DatexExpression::Text("x".to_string()),
                        DatexExpression::Literal("integer".to_owned())
                    ),
                    (
                        DatexExpression::Text("y".to_string()),
                        DatexExpression::Literal("integer".to_owned())
                    )
                ])),
                return_type: None,
                body: Box::new(DatexExpression::Statements(vec![Statement {
                    expression: DatexExpression::BinaryOperation(
                        BinaryOperator::Arithmetic(ArithmeticOperator::Add),
                        Box::new(DatexExpression::Integer(Integer::from(1))),
                        Box::new(DatexExpression::Integer(Integer::from(2))),
                        None
                    ),
                    is_terminated: true
                }])),
            }
        );
    }

    #[test]
    fn test_function_with_return_type() {
        let src = r#"
            function myFunction(x: integer) -> integer | text (
                42
            )
        "#;
        let val = parse_unwrap(src);
        assert_eq!(
            val,
            DatexExpression::FunctionDeclaration {
                name: "myFunction".to_string(),
                parameters: Box::new(DatexExpression::Map(vec![(
                    DatexExpression::Text("x".to_string()),
                    DatexExpression::Literal("integer".to_owned())
                )])),
                return_type: Some(TypeExpression::Union(vec![
                    TypeExpression::Literal("integer".to_owned()),
                    TypeExpression::Literal("text".to_owned())
                ])),
                body: Box::new(DatexExpression::Integer(Integer::from(42))),
            }
        );
    }

    #[test]
    fn type_var_declaration() {
        let src = "var x: 5 = 42";
        let val = parse_unwrap(src);
        assert_eq!(
            val,
            DatexExpression::VariableDeclaration {
                id: None,
                kind: VariableKind::Var,
                binding_mutability: BindingMutability::Mutable,
                type_annotation: Some(
                    TypeExpression::Integer(Integer::from(5)).into()
                ),
                name: "x".to_string(),
                value: Box::new(DatexExpression::Integer(Integer::from(42)))
            }
        );

        let src = "var x: integer/u8 = 42";
        let val = parse_unwrap(src);
        assert_eq!(
            val,
            DatexExpression::VariableDeclaration {
                id: None,
                kind: VariableKind::Var,
                binding_mutability: BindingMutability::Mutable,
                type_annotation: Some(TypeExpression::Literal(
                    "integer/u8".to_owned()
                )),
                name: "x".to_string(),
                value: Box::new(DatexExpression::Integer(Integer::from(42)))
            }
        );
    }

    #[deprecated(note = "Remove intersection from value syntax")]
    #[test]
    fn intersection() {
        let src = "5 & 6";
        let val = parse_unwrap(src);
        assert_eq!(
            val,
            DatexExpression::BinaryOperation(
                BinaryOperator::Logical(LogicalOperator::And),
                Box::new(DatexExpression::Integer(Integer::from(5))),
                Box::new(DatexExpression::Integer(Integer::from(6))),
                None
            )
        );

        let src = "(integer/u8 & 6) & 2";
        let val = parse_unwrap(src);
        assert_eq!(
            val,
            DatexExpression::BinaryOperation(
                BinaryOperator::Logical(LogicalOperator::And),
                Box::new(DatexExpression::BinaryOperation(
                    BinaryOperator::Logical(LogicalOperator::And),
                    Box::new(DatexExpression::BinaryOperation(
                        BinaryOperator::VariantAccess,
                        Box::new(DatexExpression::Literal(
                            "integer".to_owned()
                        )),
                        Box::new(DatexExpression::Literal("u8".to_owned())),
                        None
                    )),
                    Box::new(DatexExpression::Integer(Integer::from(6))),
                    None
                )),
                Box::new(DatexExpression::Integer(Integer::from(2))),
                None
            )
        );
    }

    #[deprecated(note = "Remove union from value syntax")]
    #[test]
    fn union() {
        let src = "5 | 6";
        let val = parse_unwrap(src);
        assert_eq!(
            val,
            DatexExpression::BinaryOperation(
                BinaryOperator::Bitwise(BitwiseOperator::Or),
                Box::new(DatexExpression::Integer(Integer::from(5))),
                Box::new(DatexExpression::Integer(Integer::from(6))),
                None
            )
        );

        let src = "(integer/u8 | 6) | 2";
        let val = parse_unwrap(src);
        assert_eq!(
            val,
            DatexExpression::BinaryOperation(
                BinaryOperator::Bitwise(BitwiseOperator::Or),
                Box::new(DatexExpression::BinaryOperation(
                    BinaryOperator::Bitwise(BitwiseOperator::Or),
                    Box::new(DatexExpression::BinaryOperation(
                        BinaryOperator::VariantAccess,
                        Box::new(DatexExpression::Literal(
                            "integer".to_owned()
                        )),
                        Box::new(DatexExpression::Literal("u8".to_owned())),
                        None
                    )),
                    Box::new(DatexExpression::Integer(Integer::from(6))),
                    None
                )),
                Box::new(DatexExpression::Integer(Integer::from(2))),
                None
            )
        );
    }

    #[test]
    fn binary_operator_precedence() {
        let src = "1 + 2 * 3";
        let val = parse_unwrap(src);
        assert_eq!(
            val,
            DatexExpression::BinaryOperation(
                BinaryOperator::Arithmetic(ArithmeticOperator::Add),
                Box::new(DatexExpression::Integer(Integer::from(1))),
                Box::new(DatexExpression::BinaryOperation(
                    BinaryOperator::Arithmetic(ArithmeticOperator::Multiply),
                    Box::new(DatexExpression::Integer(Integer::from(2))),
                    Box::new(DatexExpression::Integer(Integer::from(3))),
                    None
                )),
                None
            )
        );

        let src = "1 + 2 & 3";
        let val = parse_unwrap(src);
        assert_eq!(
            val,
            DatexExpression::BinaryOperation(
                BinaryOperator::Logical(LogicalOperator::And),
                Box::new(DatexExpression::BinaryOperation(
                    BinaryOperator::Arithmetic(ArithmeticOperator::Add),
                    Box::new(DatexExpression::Integer(Integer::from(1))),
                    Box::new(DatexExpression::Integer(Integer::from(2))),
                    None
                )),
                Box::new(DatexExpression::Integer(Integer::from(3))),
                None
            )
        );

        let src = "1 + 2 | 3";
        let val = parse_unwrap(src);
        assert_eq!(
            val,
            DatexExpression::BinaryOperation(
                BinaryOperator::Bitwise(BitwiseOperator::Or),
                Box::new(DatexExpression::BinaryOperation(
                    BinaryOperator::Arithmetic(ArithmeticOperator::Add),
                    Box::new(DatexExpression::Integer(Integer::from(1))),
                    Box::new(DatexExpression::Integer(Integer::from(2))),
                    None
                )),
                Box::new(DatexExpression::Integer(Integer::from(3))),
                None
            )
        );
    }

    #[test]
    fn generic_assessor() {
        let expected = DatexExpression::ApplyChain(
            Box::new(DatexExpression::Literal("User".to_string())),
            vec![
                ApplyOperation::GenericAccess(
                    DatexExpression::BinaryOperation(
                        BinaryOperator::VariantAccess,
                        Box::new(DatexExpression::Literal(
                            "integer".to_owned(),
                        )),
                        Box::new(DatexExpression::Literal("u8".to_owned())),
                        None,
                    ),
                ),
                ApplyOperation::FunctionCall(DatexExpression::Struct(vec![])),
            ],
        );
        assert_eq!(parse_unwrap("User<integer/u8> {}"), expected);
        assert_eq!(parse_unwrap("User< integer/u8 > {}"), expected);
        assert_eq!(parse_unwrap("User<integer/u8 > {}"), expected);
        assert!(parse("User <integer/u8> {}").is_err());
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
            let val = parse_unwrap(s);
            assert_eq!(
                val,
                DatexExpression::Conditional {
                    condition: Box::new(DatexExpression::Boolean(true)),
                    then_branch: Box::new(DatexExpression::Integer(
                        Integer::from(1)
                    )),
                    else_branch: Some(Box::new(DatexExpression::Integer(
                        Integer::from(2)
                    ))),
                }
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
            let val = parse_unwrap(s);
            assert_eq!(
                val,
                DatexExpression::Conditional {
                    condition: Box::new(DatexExpression::ComparisonOperation(
                        ComparisonOperator::StructuralEqual,
                        Box::new(DatexExpression::BinaryOperation(
                            BinaryOperator::Arithmetic(ArithmeticOperator::Add),
                            Box::new(DatexExpression::Boolean(true)),
                            Box::new(DatexExpression::Integer(Integer::from(
                                1
                            ))),
                            None
                        )),
                        Box::new(DatexExpression::Integer(Integer::from(2)))
                    )),
                    then_branch: Box::new(DatexExpression::Integer(
                        Integer::from(4)
                    )),
                    else_branch: Some(Box::new(DatexExpression::Integer(
                        Integer::from(2)
                    ))),
                }
            );
        }

        // make sure apply chains still work
        let src = vec![
            "if true + 1 == 2 test [1,2,3]",
            "if true + 1 == 2 (test [1,2,3])",
        ];
        for s in src {
            let val = parse_unwrap(s);
            assert_eq!(
                val,
                DatexExpression::Conditional {
                    condition: Box::new(DatexExpression::ComparisonOperation(
                        ComparisonOperator::StructuralEqual,
                        Box::new(DatexExpression::BinaryOperation(
                            BinaryOperator::Arithmetic(ArithmeticOperator::Add),
                            Box::new(DatexExpression::Boolean(true)),
                            Box::new(DatexExpression::Integer(Integer::from(
                                1
                            ))),
                            None
                        )),
                        Box::new(DatexExpression::Integer(Integer::from(2)))
                    )),
                    then_branch: Box::new(DatexExpression::ApplyChain(
                        Box::new(DatexExpression::Literal("test".to_string())),
                        vec![ApplyOperation::FunctionCall(
                            DatexExpression::Array(vec![
                                DatexExpression::Integer(Integer::from(1)),
                                DatexExpression::Integer(Integer::from(2)),
                                DatexExpression::Integer(Integer::from(3)),
                            ])
                        )]
                    )),
                    else_branch: None,
                }
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
            ) else null;
        "#;

        let val = parse_unwrap(src);
        assert_eq!(
            val,
            DatexExpression::Conditional {
                condition: Box::new(DatexExpression::ComparisonOperation(
                    ComparisonOperator::StructuralEqual,
                    Box::new(DatexExpression::Literal("x".to_string())),
                    Box::new(DatexExpression::Integer(Integer::from(4)))
                )),
                then_branch: Box::new(DatexExpression::Text("4".to_string())),
                else_branch: Some(Box::new(DatexExpression::Conditional {
                    condition: Box::new(DatexExpression::ComparisonOperation(
                        ComparisonOperator::StructuralEqual,
                        Box::new(DatexExpression::Literal("x".to_string())),
                        Box::new(DatexExpression::Text("hello".to_string()))
                    )),
                    then_branch: Box::new(DatexExpression::Text(
                        "42".to_string()
                    )),
                    else_branch: Some(Box::new(DatexExpression::Null))
                })),
            }
        );
    }

    #[test]
    fn unary_operator() {
        let src = "+(User {})";
        let val = parse_unwrap(src);
        assert_eq!(
            val,
            DatexExpression::UnaryOperation(
                UnaryOperator::Plus,
                Box::new(DatexExpression::ApplyChain(
                    Box::new(DatexExpression::Literal("User".to_string())),
                    vec![ApplyOperation::FunctionCall(
                        DatexExpression::Struct(vec![])
                    )]
                )),
            )
        );

        let src = "-(5)";
        let val = parse_unwrap(src);
        assert_eq!(
            val,
            DatexExpression::UnaryOperation(
                UnaryOperator::Minus,
                Box::new(DatexExpression::Integer(Integer::from(5)))
            )
        );

        let src = "+-+-myVal";
        let val = parse_unwrap(src);
        assert_eq!(
            val,
            DatexExpression::UnaryOperation(
                UnaryOperator::Plus,
                Box::new(DatexExpression::UnaryOperation(
                    UnaryOperator::Minus,
                    Box::new(DatexExpression::UnaryOperation(
                        UnaryOperator::Plus,
                        Box::new(DatexExpression::UnaryOperation(
                            UnaryOperator::Minus,
                            Box::new(DatexExpression::Literal(
                                "myVal".to_string()
                            ))
                        ))
                    ))
                ))
            )
        );
    }

    #[test]
    fn var_declaration_with_type_simple() {
        let src = "var x: integer = 42";
        let val = parse_unwrap(src);
        assert_eq!(
            val,
            DatexExpression::VariableDeclaration {
                id: None,
                kind: VariableKind::Var,
                binding_mutability: BindingMutability::Mutable,
                type_annotation: Some(TypeExpression::Literal(
                    "integer".to_string()
                )),
                name: "x".to_string(),
                value: Box::new(DatexExpression::Integer(Integer::from(42)))
            }
        );

        let src = "var x: User = 42";
        let val = parse_unwrap(src);
        assert_eq!(
            val,
            DatexExpression::VariableDeclaration {
                id: None,
                kind: VariableKind::Var,
                binding_mutability: BindingMutability::Mutable,
                type_annotation: Some(TypeExpression::Literal(
                    "User".to_string()
                )),
                name: "x".to_string(),
                value: Box::new(DatexExpression::Integer(Integer::from(42)))
            }
        );

        let src = "var x: integer/u8 = 42";
        let val = parse_unwrap(src);
        assert_eq!(
            val,
            DatexExpression::VariableDeclaration {
                id: None,
                kind: VariableKind::Var,
                binding_mutability: BindingMutability::Mutable,
                type_annotation: Some(TypeExpression::Literal(
                    "integer/u8".to_owned()
                )),
                name: "x".to_string(),
                value: Box::new(DatexExpression::Integer(Integer::from(42)))
            }
        );
    }

    #[test]
    fn var_declaration_with_type_union() {
        let src = "var x: integer/u8 | text = 42";
        let val = parse_unwrap(src);
        assert_eq!(
            val,
            DatexExpression::VariableDeclaration {
                id: None,
                kind: VariableKind::Var,
                binding_mutability: BindingMutability::Mutable,
                type_annotation: Some(TypeExpression::Union(vec![
                    TypeExpression::Literal("integer/u8".to_owned()),
                    TypeExpression::Literal("text".to_owned())
                ])),
                name: "x".to_string(),
                value: Box::new(DatexExpression::Integer(Integer::from(42)))
            }
        );
    }

    #[test]
    fn var_declaration_with_type_intersection() {
        let src = "var x: 5 & 6 = 42";
        let val = parse_unwrap(src);
        assert_eq!(
            val,
            DatexExpression::VariableDeclaration {
                id: None,
                kind: VariableKind::Var,
                binding_mutability: BindingMutability::Mutable,
                type_annotation: Some(TypeExpression::Intersection(vec![
                    TypeExpression::Integer(Integer::from(5)),
                    TypeExpression::Integer(Integer::from(6))
                ])),
                name: "x".to_string(),
                value: Box::new(DatexExpression::Integer(Integer::from(42)))
            }
        );
    }

    #[test]
    fn test_type_var_declaration_array() {
        let src = "var x: integer[] = 42";
        let val = parse_unwrap(src);
        assert_eq!(
            val,
            DatexExpression::VariableDeclaration {
                id: None,
                kind: VariableKind::Var,
                binding_mutability: BindingMutability::Mutable,
                type_annotation: Some(TypeExpression::SliceArray(Box::new(
                    TypeExpression::Literal("integer".to_owned())
                ))),
                name: "x".to_string(),
                value: Box::new(DatexExpression::Integer(Integer::from(42)))
            }
        );
    }

    #[test]
    fn equal_operators() {
        let src = "3 == 1 + 2";
        let val = parse_unwrap(src);
        assert_eq!(
            val,
            DatexExpression::ComparisonOperation(
                ComparisonOperator::StructuralEqual,
                Box::new(DatexExpression::Integer(Integer::from(3))),
                Box::new(DatexExpression::BinaryOperation(
                    BinaryOperator::Arithmetic(ArithmeticOperator::Add),
                    Box::new(DatexExpression::Integer(Integer::from(1))),
                    Box::new(DatexExpression::Integer(Integer::from(2))),
                    None
                ))
            )
        );

        let src = "3 === 1 + 2";
        let val = parse_unwrap(src);
        assert_eq!(
            val,
            DatexExpression::ComparisonOperation(
                ComparisonOperator::Equal,
                Box::new(DatexExpression::Integer(Integer::from(3))),
                Box::new(DatexExpression::BinaryOperation(
                    BinaryOperator::Arithmetic(ArithmeticOperator::Add),
                    Box::new(DatexExpression::Integer(Integer::from(1))),
                    Box::new(DatexExpression::Integer(Integer::from(2))),
                    None
                ))
            )
        );

        let src = "5 != 1 + 2";
        let val = parse_unwrap(src);
        assert_eq!(
            val,
            DatexExpression::ComparisonOperation(
                ComparisonOperator::NotStructuralEqual,
                Box::new(DatexExpression::Integer(Integer::from(5))),
                Box::new(DatexExpression::BinaryOperation(
                    BinaryOperator::Arithmetic(ArithmeticOperator::Add),
                    Box::new(DatexExpression::Integer(Integer::from(1))),
                    Box::new(DatexExpression::Integer(Integer::from(2))),
                    None
                ))
            )
        );
        let src = "5 !== 1 + 2";
        let val = parse_unwrap(src);
        assert_eq!(
            val,
            DatexExpression::ComparisonOperation(
                ComparisonOperator::NotEqual,
                Box::new(DatexExpression::Integer(Integer::from(5))),
                Box::new(DatexExpression::BinaryOperation(
                    BinaryOperator::Arithmetic(ArithmeticOperator::Add),
                    Box::new(DatexExpression::Integer(Integer::from(1))),
                    Box::new(DatexExpression::Integer(Integer::from(2))),
                    None
                ))
            )
        );

        let src = "5 is 1 + 2";
        let val = parse_unwrap(src);
        assert_eq!(
            val,
            DatexExpression::ComparisonOperation(
                ComparisonOperator::Is,
                Box::new(DatexExpression::Integer(Integer::from(5))),
                Box::new(DatexExpression::BinaryOperation(
                    BinaryOperator::Arithmetic(ArithmeticOperator::Add),
                    Box::new(DatexExpression::Integer(Integer::from(1))),
                    Box::new(DatexExpression::Integer(Integer::from(2))),
                    None
                ))
            )
        );
    }

    #[test]
    fn null() {
        let src = "null";
        let val = parse_unwrap(src);
        assert_eq!(val, DatexExpression::Null);
    }

    #[test]
    fn boolean() {
        let src_true = "true";
        let val_true = parse_unwrap(src_true);
        assert_eq!(val_true, DatexExpression::Boolean(true));

        let src_false = "false";
        let val_false = parse_unwrap(src_false);
        assert_eq!(val_false, DatexExpression::Boolean(false));
    }

    #[test]
    fn integer() {
        let src = "123456789123456789";
        let num = parse_unwrap(src);
        assert_eq!(
            num,
            DatexExpression::Integer(
                Integer::from_string("123456789123456789").unwrap()
            )
        );
    }

    #[test]
    fn negative_integer() {
        let src = "-123456789123456789";
        let num = parse_unwrap(src);
        assert_eq!(
            num,
            DatexExpression::Integer(
                Integer::from_string("-123456789123456789").unwrap()
            )
        );
    }

    #[test]
    fn integer_with_underscores() {
        let src = "123_456";
        let num = parse_unwrap(src);
        assert_eq!(
            num,
            DatexExpression::Integer(Integer::from_string("123456").unwrap())
        );
    }

    #[test]
    fn hex_integer() {
        let src = "0x1A2B3C4D5E6F";
        let num = parse_unwrap(src);
        assert_eq!(
            num,
            DatexExpression::Integer(
                Integer::from_string_radix("1A2B3C4D5E6F", 16).unwrap()
            )
        );
    }

    #[test]
    fn octal_integer() {
        let src = "0o755";
        let num = parse_unwrap(src);
        assert_eq!(
            num,
            DatexExpression::Integer(
                Integer::from_string_radix("755", 8).unwrap()
            )
        );
    }

    #[test]
    fn binary_integer() {
        let src = "0b101010";
        let num = parse_unwrap(src);
        assert_eq!(
            num,
            DatexExpression::Integer(
                Integer::from_string_radix("101010", 2).unwrap()
            )
        );
    }

    #[test]
    fn integer_with_exponent() {
        let src = "2e10";
        let num = parse_unwrap(src);
        assert_eq!(
            num,
            DatexExpression::Decimal(
                Decimal::from_string("20000000000").unwrap()
            )
        );
    }

    #[test]
    fn decimal() {
        let src = "123.456789123456";
        let num = parse_unwrap(src);
        assert_eq!(
            num,
            DatexExpression::Decimal(
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
            ("+1_000.0", "1000.0"),
        ];

        for (src, expected_str) in cases {
            let num = parse_unwrap(src);
            assert_eq!(
                num,
                DatexExpression::Decimal(
                    Decimal::from_string(expected_str).unwrap()
                ),
                "Failed to parse: {src}"
            );
        }
    }

    #[test]
    fn negative_decimal() {
        let src = "-123.4";
        let num = parse_unwrap(src);
        assert_eq!(
            num,
            DatexExpression::Decimal(Decimal::from_string("-123.4").unwrap())
        );
    }

    #[test]
    fn decimal_with_exponent() {
        let src = "1.23456789123456e2";
        let num = parse_unwrap(src);
        assert_eq!(
            num,
            DatexExpression::Decimal(
                Decimal::from_string("123.456789123456").unwrap()
            )
        );
    }

    #[test]
    fn decimal_with_negative_exponent() {
        let src = "1.23456789123456e-2";
        let num = parse_unwrap(src);
        assert_eq!(
            num,
            DatexExpression::Decimal(
                Decimal::from_string("0.0123456789123456").unwrap()
            )
        );
    }

    #[test]
    fn decimal_with_positive_exponent() {
        let src = "1.23456789123456E+2";
        let num = parse_unwrap(src);
        assert_eq!(
            num,
            DatexExpression::Decimal(
                Decimal::from_string("123.456789123456").unwrap()
            )
        );
    }

    #[test]
    fn decimal_with_trailing_point() {
        let src = "123.";
        let num = parse_unwrap(src);
        assert_eq!(
            num,
            DatexExpression::Decimal(Decimal::from_string("123.0").unwrap())
        );
    }

    #[test]
    fn decimal_with_leading_point() {
        let src = ".456789123456";
        let num = parse_unwrap(src);
        assert_eq!(
            num,
            DatexExpression::Decimal(
                Decimal::from_string("0.456789123456").unwrap()
            )
        );

        let src = ".423e-2";
        let num = parse_unwrap(src);
        assert_eq!(
            num,
            DatexExpression::Decimal(Decimal::from_string("0.00423").unwrap())
        );
    }

    #[test]
    fn text_double_quotes() {
        let src = r#""Hello, world!""#;
        let text = parse_unwrap(src);
        assert_eq!(text, DatexExpression::Text("Hello, world!".to_string()));
    }

    #[test]
    fn text_single_quotes() {
        let src = r#"'Hello, world!'"#;
        let text = parse_unwrap(src);
        assert_eq!(text, DatexExpression::Text("Hello, world!".to_string()));
    }

    #[test]
    fn text_escape_sequences() {
        let src =
            r#""Hello, \"world\"! \n New line \t tab \uD83D\uDE00 \u2764""#;
        let text = parse_unwrap(src);

        assert_eq!(
            text,
            DatexExpression::Text(
                "Hello, \"world\"! \n New line \t tab 😀 ❤".to_string()
            )
        );
    }

    #[test]
    fn text_escape_sequences_2() {
        let src =
            r#""\u0048\u0065\u006C\u006C\u006F, \u2764\uFE0F, \uD83D\uDE00""#;
        let text = parse_unwrap(src);
        assert_eq!(text, DatexExpression::Text("Hello, ❤️, 😀".to_string()));
    }

    #[test]
    fn text_nested_escape_sequences() {
        let src = r#""\\\\""#;
        let text = parse_unwrap(src);
        assert_eq!(text, DatexExpression::Text("\\\\".to_string()));
    }

    #[test]
    fn text_nested_escape_sequences_2() {
        let src = r#""\\\"""#;
        let text = parse_unwrap(src);
        assert_eq!(text, DatexExpression::Text("\\\"".to_string()));
    }

    #[test]
    fn empty_array() {
        let src = "[]";
        let arr = parse_unwrap(src);
        assert_eq!(arr, DatexExpression::Array(vec![]));
    }

    #[test]
    fn array_with_values() {
        let src = "[1, 2, 3, 4.5, \"text\"]";
        let arr = parse_unwrap(src);

        assert_eq!(
            arr,
            DatexExpression::Array(vec![
                DatexExpression::Integer(Integer::from(1)),
                DatexExpression::Integer(Integer::from(2)),
                DatexExpression::Integer(Integer::from(3)),
                DatexExpression::Decimal(Decimal::from_string("4.5").unwrap()),
                DatexExpression::Text("text".to_string()),
            ])
        );
    }

    #[test]
    fn empty_object() {
        let src = "{}";
        let obj = parse_unwrap(src);

        assert_eq!(obj, DatexExpression::Struct(vec![]));
    }

    #[test]
    fn list() {
        let src = "1,2";
        let list = parse_unwrap(src);

        assert_eq!(
            list,
            DatexExpression::List(vec![
                DatexExpression::Integer(Integer::from(1)),
                DatexExpression::Integer(Integer::from(2)),
            ])
        );
    }

    #[test]
    fn scoped_list() {
        let src = "(1, 2)";
        let list = parse_unwrap(src);

        assert_eq!(
            list,
            DatexExpression::List(vec![
                DatexExpression::Integer(Integer::from(1)),
                DatexExpression::Integer(Integer::from(2)),
            ])
        );
    }

    #[test]
    fn map_without_parentheses() {
        let src = "1: 2, 3: 4, xy:2, 'a b c': 'd'";
        let map = parse_unwrap(src);

        assert_eq!(
            map,
            DatexExpression::Map(vec![
                (
                    DatexExpression::Integer(Integer::from(1)),
                    DatexExpression::Integer(Integer::from(2))
                ),
                (
                    DatexExpression::Integer(Integer::from(3)),
                    DatexExpression::Integer(Integer::from(4))
                ),
                (
                    DatexExpression::Text("xy".to_string()),
                    DatexExpression::Integer(Integer::from(2))
                ),
                (
                    DatexExpression::Text("a b c".to_string()),
                    DatexExpression::Text("d".to_string())
                ),
            ])
        );
    }

    #[test]
    fn list_array() {
        let src = "[(1,2),3,(4,)]";
        let arr = parse_unwrap(src);

        assert_eq!(
            arr,
            DatexExpression::Array(vec![
                DatexExpression::List(vec![
                    DatexExpression::Integer(Integer::from(1)),
                    DatexExpression::Integer(Integer::from(2)),
                ]),
                DatexExpression::Integer(Integer::from(3)),
                DatexExpression::List(vec![DatexExpression::Integer(
                    Integer::from(4)
                )]),
            ])
        );
    }

    #[test]
    fn single_value_list() {
        let src = "1,";
        let list = parse_unwrap(src);

        assert_eq!(
            list,
            DatexExpression::List(vec![DatexExpression::Integer(
                Integer::from(1)
            )])
        );
    }

    #[test]
    fn single_entry_map() {
        let src = "x: 1";
        let map = parse_unwrap(src);
        assert_eq!(
            map,
            DatexExpression::Map(vec![(
                DatexExpression::Text("x".to_string()),
                DatexExpression::Integer(Integer::from(1))
            )])
        );
    }

    #[test]
    fn scoped_atom() {
        let src = "(1)";
        let atom = parse_unwrap(src);
        assert_eq!(atom, DatexExpression::Integer(Integer::from(1)));
    }

    #[test]
    fn scoped_array() {
        let src = "(([1, 2, 3]))";
        let arr = parse_unwrap(src);

        assert_eq!(
            arr,
            DatexExpression::Array(vec![
                DatexExpression::Integer(Integer::from(1)),
                DatexExpression::Integer(Integer::from(2)),
                DatexExpression::Integer(Integer::from(3)),
            ])
        );
    }

    #[test]
    fn object_with_key_value_pairs() {
        let src = r#"{"key1": "value1", "key2": 42, "key3": true}"#;
        let obj = parse_unwrap(src);

        assert_eq!(
            obj,
            DatexExpression::Struct(vec![
                (
                    "key1".to_string(),
                    DatexExpression::Text("value1".to_string())
                ),
                (
                    "key2".to_string(),
                    DatexExpression::Integer(Integer::from(42))
                ),
                ("key3".to_string(), DatexExpression::Boolean(true)),
            ])
        );
    }

    #[test]
    fn dynamic_map_keys() {
        let src = r#"((1): "value1", (2): 42, (3): true)"#;
        let obj = parse_unwrap(src);
        assert_eq!(
            obj,
            DatexExpression::Map(vec![
                (
                    DatexExpression::Integer(Integer::from(1)),
                    DatexExpression::Text("value1".to_string())
                ),
                (
                    DatexExpression::Integer(Integer::from(2)),
                    DatexExpression::Integer(Integer::from(42))
                ),
                (
                    DatexExpression::Integer(Integer::from(3)),
                    DatexExpression::Boolean(true)
                ),
            ])
        );
    }

    #[test]
    fn add() {
        // Test with escaped characters in text
        let src = "1 + 2";
        let expr = parse_unwrap(src);
        assert_eq!(
            expr,
            DatexExpression::BinaryOperation(
                BinaryOperator::Arithmetic(ArithmeticOperator::Add),
                Box::new(DatexExpression::Integer(Integer::from(1))),
                Box::new(DatexExpression::Integer(Integer::from(2))),
                None
            )
        );
    }

    #[test]
    fn add_complex_values() {
        // Test with escaped characters in text
        let src = "[] + x + (1 + 2)";
        let expr = parse_unwrap(src);
        assert_eq!(
            expr,
            DatexExpression::BinaryOperation(
                BinaryOperator::Arithmetic(ArithmeticOperator::Add),
                Box::new(DatexExpression::BinaryOperation(
                    BinaryOperator::Arithmetic(ArithmeticOperator::Add),
                    Box::new(DatexExpression::Array(vec![])),
                    Box::new(DatexExpression::Literal("x".to_string())),
                    None
                )),
                Box::new(DatexExpression::BinaryOperation(
                    BinaryOperator::Arithmetic(ArithmeticOperator::Add),
                    Box::new(DatexExpression::Integer(Integer::from(1))),
                    Box::new(DatexExpression::Integer(Integer::from(2))),
                    None
                )),
                None
            )
        );
    }

    #[test]
    fn subtract() {
        let src = "5 - 3";
        let expr = parse_unwrap(src);
        assert_eq!(
            expr,
            DatexExpression::BinaryOperation(
                BinaryOperator::Arithmetic(ArithmeticOperator::Subtract),
                Box::new(DatexExpression::Integer(Integer::from(5))),
                Box::new(DatexExpression::Integer(Integer::from(3))),
                None
            )
        );
    }

    #[test]
    fn multiply() {
        let src = "4 * 2";
        let expr = parse_unwrap(src);
        assert_eq!(
            expr,
            DatexExpression::BinaryOperation(
                BinaryOperator::Arithmetic(ArithmeticOperator::Multiply),
                Box::new(DatexExpression::Integer(Integer::from(4))),
                Box::new(DatexExpression::Integer(Integer::from(2))),
                None
            )
        );
    }

    #[test]
    fn divide() {
        let src = "8 / 2";
        let expr = parse_unwrap(src);
        assert_eq!(
            expr,
            DatexExpression::BinaryOperation(
                BinaryOperator::Arithmetic(ArithmeticOperator::Divide),
                Box::new(DatexExpression::Integer(Integer::from(8))),
                Box::new(DatexExpression::Integer(Integer::from(2))),
                None
            )
        );

        let src = "8 /2";
        let expr = parse_unwrap(src);
        assert_eq!(
            expr,
            DatexExpression::BinaryOperation(
                BinaryOperator::Arithmetic(ArithmeticOperator::Divide),
                Box::new(DatexExpression::Integer(Integer::from(8))),
                Box::new(DatexExpression::Integer(Integer::from(2))),
                None
            )
        );

        let src = "8u8/2";
        let expr = parse_unwrap(src);
        assert_eq!(
            expr,
            DatexExpression::BinaryOperation(
                BinaryOperator::Arithmetic(ArithmeticOperator::Divide),
                Box::new(DatexExpression::TypedInteger(TypedInteger::from(
                    8u8
                ))),
                Box::new(DatexExpression::Integer(Integer::from(2))),
                None
            )
        );
    }

    #[test]
    fn complex_calculation() {
        let src = "1 + 2 * 3 + 4";
        let expr = parse_unwrap(src);
        assert_eq!(
            expr,
            DatexExpression::BinaryOperation(
                BinaryOperator::Arithmetic(ArithmeticOperator::Add),
                Box::new(DatexExpression::BinaryOperation(
                    BinaryOperator::Arithmetic(ArithmeticOperator::Add),
                    Box::new(DatexExpression::Integer(Integer::from(1))),
                    Box::new(DatexExpression::BinaryOperation(
                        BinaryOperator::Arithmetic(
                            ArithmeticOperator::Multiply
                        ),
                        Box::new(DatexExpression::Integer(Integer::from(2))),
                        Box::new(DatexExpression::Integer(Integer::from(3))),
                        None
                    )),
                    None
                )),
                Box::new(DatexExpression::Integer(Integer::from(4))),
                None
            )
        );
    }

    #[test]
    fn nested_addition() {
        let src = "1 + (2 + 3)";
        let expr = parse_unwrap(src);
        assert_eq!(
            expr,
            DatexExpression::BinaryOperation(
                BinaryOperator::Arithmetic(ArithmeticOperator::Add),
                Box::new(DatexExpression::Integer(Integer::from(1))),
                Box::new(DatexExpression::BinaryOperation(
                    BinaryOperator::Arithmetic(ArithmeticOperator::Add),
                    Box::new(DatexExpression::Integer(Integer::from(2))),
                    Box::new(DatexExpression::Integer(Integer::from(3))),
                    None
                )),
                None
            )
        );
    }

    #[test]
    fn add_statements_1() {
        // Test with escaped characters in text
        let src = "1 + (2;3)";
        let expr = parse_unwrap(src);
        assert_eq!(
            expr,
            DatexExpression::BinaryOperation(
                BinaryOperator::Arithmetic(ArithmeticOperator::Add),
                Box::new(DatexExpression::Integer(Integer::from(1))),
                Box::new(DatexExpression::Statements(vec![
                    Statement {
                        expression: DatexExpression::Integer(Integer::from(2)),
                        is_terminated: true,
                    },
                    Statement {
                        expression: DatexExpression::Integer(Integer::from(3)),
                        is_terminated: false,
                    },
                ])),
                None
            )
        );
    }

    #[test]
    fn add_statements_2() {
        // Test with escaped characters in text
        let src = "(1;2) + 3";
        let expr = parse_unwrap(src);
        assert_eq!(
            expr,
            DatexExpression::BinaryOperation(
                BinaryOperator::Arithmetic(ArithmeticOperator::Add),
                Box::new(DatexExpression::Statements(vec![
                    Statement {
                        expression: DatexExpression::Integer(Integer::from(1)),
                        is_terminated: true,
                    },
                    Statement {
                        expression: DatexExpression::Integer(Integer::from(2)),
                        is_terminated: false,
                    },
                ])),
                Box::new(DatexExpression::Integer(Integer::from(3))),
                None
            )
        );
    }

    #[test]
    fn nested_expressions() {
        let src = "[1 + 2]";
        let expr = parse_unwrap(src);
        assert_eq!(
            expr,
            DatexExpression::Array(vec![DatexExpression::BinaryOperation(
                BinaryOperator::Arithmetic(ArithmeticOperator::Add),
                Box::new(DatexExpression::Integer(Integer::from(1))),
                Box::new(DatexExpression::Integer(Integer::from(2))),
                None
            )])
        );
    }

    #[test]
    fn multi_statement_expression() {
        let src = "1;2";
        let expr = parse_unwrap(src);
        assert_eq!(
            expr,
            DatexExpression::Statements(vec![
                Statement {
                    expression: DatexExpression::Integer(Integer::from(1)),
                    is_terminated: true,
                },
                Statement {
                    expression: DatexExpression::Integer(Integer::from(2)),
                    is_terminated: false,
                },
            ])
        );
    }

    #[test]
    fn nested_scope_statements() {
        let src = "(1; 2; 3)";
        let expr = parse_unwrap(src);
        assert_eq!(
            expr,
            DatexExpression::Statements(vec![
                Statement {
                    expression: DatexExpression::Integer(Integer::from(1)),
                    is_terminated: true,
                },
                Statement {
                    expression: DatexExpression::Integer(Integer::from(2)),
                    is_terminated: true,
                },
                Statement {
                    expression: DatexExpression::Integer(Integer::from(3)),
                    is_terminated: false,
                },
            ])
        );
    }
    #[test]
    fn nested_scope_statements_closed() {
        let src = "(1; 2; 3;)";
        let expr = parse_unwrap(src);
        assert_eq!(
            expr,
            DatexExpression::Statements(vec![
                Statement {
                    expression: DatexExpression::Integer(Integer::from(1)),
                    is_terminated: true,
                },
                Statement {
                    expression: DatexExpression::Integer(Integer::from(2)),
                    is_terminated: true,
                },
                Statement {
                    expression: DatexExpression::Integer(Integer::from(3)),
                    is_terminated: true,
                },
            ])
        );
    }

    #[test]
    fn nested_statements_in_object() {
        let src = r#"{"key": (1; 2; 3)}"#;
        let expr = parse_unwrap(src);
        assert_eq!(
            expr,
            DatexExpression::Struct(vec![(
                "key".to_string(),
                DatexExpression::Statements(vec![
                    Statement {
                        expression: DatexExpression::Integer(Integer::from(1)),
                        is_terminated: true,
                    },
                    Statement {
                        expression: DatexExpression::Integer(Integer::from(2)),
                        is_terminated: true,
                    },
                    Statement {
                        expression: DatexExpression::Integer(Integer::from(3)),
                        is_terminated: false,
                    },
                ])
            ),])
        );
    }

    #[test]
    fn single_statement() {
        let src = "1;";
        let expr = parse_unwrap(src);
        assert_eq!(
            expr,
            DatexExpression::Statements(vec![Statement {
                expression: DatexExpression::Integer(Integer::from(1)),
                is_terminated: true,
            },])
        );
    }

    #[test]
    fn empty_statement() {
        let src = ";";
        let expr = parse_unwrap(src);
        assert_eq!(expr, DatexExpression::Statements(vec![]));
    }

    #[test]
    fn empty_statement_multiple() {
        let src = ";;;";
        let expr = parse_unwrap(src);
        assert_eq!(expr, DatexExpression::Statements(vec![]));
    }

    #[test]
    fn variable_expression() {
        let src = "myVar";
        let expr = parse_unwrap(src);
        assert_eq!(expr, DatexExpression::Literal("myVar".to_string()));
    }

    #[test]
    fn variable_expression_with_operations() {
        let src = "myVar + 1";
        let expr = parse_unwrap(src);
        assert_eq!(
            expr,
            DatexExpression::BinaryOperation(
                BinaryOperator::Arithmetic(ArithmeticOperator::Add),
                Box::new(DatexExpression::Literal("myVar".to_string())),
                Box::new(DatexExpression::Integer(Integer::from(1))),
                None
            )
        );
    }

    #[test]
    fn apply_expression() {
        let src = "myFunc(1, 2, 3)";
        let expr = parse_unwrap(src);
        assert_eq!(
            expr,
            DatexExpression::ApplyChain(
                Box::new(DatexExpression::Literal("myFunc".to_string())),
                vec![ApplyOperation::FunctionCall(DatexExpression::List(
                    vec![
                        DatexExpression::Integer(Integer::from(1)),
                        DatexExpression::Integer(Integer::from(2)),
                        DatexExpression::Integer(Integer::from(3)),
                    ]
                ),)],
            )
        );
    }

    #[test]
    fn apply_empty() {
        let src = "myFunc()";
        let expr = parse_unwrap(src);
        assert_eq!(
            expr,
            DatexExpression::ApplyChain(
                Box::new(DatexExpression::Literal("myFunc".to_string())),
                vec![ApplyOperation::FunctionCall(
                    DatexExpression::Statements(vec![])
                )],
            )
        );
    }

    #[test]
    fn apply_multiple() {
        let src = "myFunc(1)(2, 3)";
        let expr = parse_unwrap(src);
        assert_eq!(
            expr,
            DatexExpression::ApplyChain(
                Box::new(DatexExpression::Literal("myFunc".to_string())),
                vec![
                    ApplyOperation::FunctionCall(DatexExpression::Integer(
                        Integer::from(1)
                    ),),
                    ApplyOperation::FunctionCall(DatexExpression::List(vec![
                        DatexExpression::Integer(Integer::from(2)),
                        DatexExpression::Integer(Integer::from(3)),
                    ]))
                ],
            )
        );
    }

    #[test]
    fn apply_atom() {
        let src = "print 'test'";
        let expr = parse_unwrap(src);
        assert_eq!(
            expr,
            DatexExpression::ApplyChain(
                Box::new(DatexExpression::Literal("print".to_string())),
                vec![ApplyOperation::FunctionCall(DatexExpression::Text(
                    "test".to_string()
                ))],
            )
        );
    }

    #[test]
    fn property_access() {
        let src = "myObj.myProp";
        let expr = parse_unwrap(src);
        assert_eq!(
            expr,
            DatexExpression::ApplyChain(
                Box::new(DatexExpression::Literal("myObj".to_string())),
                vec![ApplyOperation::PropertyAccess(DatexExpression::Text(
                    "myProp".to_string()
                ))],
            )
        );
    }

    #[test]
    fn property_access_scoped() {
        let src = "myObj.(1)";
        let expr = parse_unwrap(src);
        assert_eq!(
            expr,
            DatexExpression::ApplyChain(
                Box::new(DatexExpression::Literal("myObj".to_string())),
                vec![ApplyOperation::PropertyAccess(DatexExpression::Integer(
                    Integer::from(1)
                ))],
            )
        );
    }

    #[test]
    fn property_access_multiple() {
        let src = "myObj.myProp.anotherProp.(1 + 2).(x;y)";
        let expr = parse_unwrap(src);
        assert_eq!(
            expr,
            DatexExpression::ApplyChain(
                Box::new(DatexExpression::Literal("myObj".to_string())),
                vec![
                    ApplyOperation::PropertyAccess(DatexExpression::Text(
                        "myProp".to_string()
                    )),
                    ApplyOperation::PropertyAccess(DatexExpression::Text(
                        "anotherProp".to_string()
                    )),
                    ApplyOperation::PropertyAccess(
                        DatexExpression::BinaryOperation(
                            BinaryOperator::Arithmetic(ArithmeticOperator::Add),
                            Box::new(DatexExpression::Integer(Integer::from(
                                1
                            ))),
                            Box::new(DatexExpression::Integer(Integer::from(
                                2
                            ))),
                            None
                        )
                    ),
                    ApplyOperation::PropertyAccess(
                        DatexExpression::Statements(vec![
                            Statement {
                                expression: DatexExpression::Literal(
                                    "x".to_string()
                                ),
                                is_terminated: true,
                            },
                            Statement {
                                expression: DatexExpression::Literal(
                                    "y".to_string()
                                ),
                                is_terminated: false,
                            },
                        ])
                    ),
                ],
            )
        );
    }

    #[test]
    fn property_access_and_apply() {
        let src = "myObj.myProp(1, 2)";
        let expr = parse_unwrap(src);
        assert_eq!(
            expr,
            DatexExpression::ApplyChain(
                Box::new(DatexExpression::Literal("myObj".to_string())),
                vec![
                    ApplyOperation::PropertyAccess(DatexExpression::Text(
                        "myProp".to_string()
                    )),
                    ApplyOperation::FunctionCall(DatexExpression::List(vec![
                        DatexExpression::Integer(Integer::from(1)),
                        DatexExpression::Integer(Integer::from(2)),
                    ])),
                ],
            )
        );
    }

    #[test]
    fn apply_and_property_access() {
        let src = "myFunc(1).myProp";
        let expr = parse_unwrap(src);
        assert_eq!(
            expr,
            DatexExpression::ApplyChain(
                Box::new(DatexExpression::Literal("myFunc".to_string())),
                vec![
                    ApplyOperation::FunctionCall(DatexExpression::Integer(
                        Integer::from(1)
                    )),
                    ApplyOperation::PropertyAccess(DatexExpression::Text(
                        "myProp".to_string()
                    )),
                ],
            )
        );
    }

    #[test]
    fn nested_apply_and_property_access() {
        let src = "((x(1)).y).z";
        let expr = parse_unwrap(src);
        assert_eq!(
            expr,
            DatexExpression::ApplyChain(
                Box::new(DatexExpression::ApplyChain(
                    Box::new(DatexExpression::ApplyChain(
                        Box::new(DatexExpression::Literal("x".to_string())),
                        vec![ApplyOperation::FunctionCall(
                            DatexExpression::Integer(Integer::from(1))
                        )],
                    )),
                    vec![ApplyOperation::PropertyAccess(
                        DatexExpression::Text("y".to_string())
                    )],
                )),
                vec![ApplyOperation::PropertyAccess(DatexExpression::Text(
                    "z".to_string()
                ))],
            )
        );
    }

    #[test]
    fn type_declaration_statement() {
        let src = "type User = { age: 42, name: \"John\" };";
        let expr = parse_unwrap(src);
        assert_eq!(
            expr,
            DatexExpression::Statements(vec![Statement {
                expression: DatexExpression::TypeDeclaration {
                    id: None,
                    name: "User".to_string(),
                    value: TypeExpression::Struct(vec![
                        (
                            "age".to_string(),
                            TypeExpression::Integer(Integer::from(42))
                        ),
                        (
                            "name".to_string(),
                            TypeExpression::Text("John".to_string())
                        ),
                    ]),
                },
                is_terminated: true,
            },])
        );

        // make sure { type: 42, name: "John" } is not parsed as type declaration
        let src = r#"{ type: 42, name: "John" };"#;
        let expr = parse_unwrap(src);
        assert_eq!(
            expr,
            DatexExpression::Statements(vec![Statement {
                expression: DatexExpression::Struct(vec![
                    (
                        "type".to_string(),
                        DatexExpression::Integer(Integer::from(42))
                    ),
                    (
                        "name".to_string(),
                        DatexExpression::Text("John".to_string())
                    ),
                ]),
                is_terminated: true,
            },])
        );
    }

    #[test]
    fn variable_declaration_statement() {
        let src = "const x = 42;";
        let expr = parse_unwrap(src);
        assert_eq!(
            expr,
            DatexExpression::Statements(vec![Statement {
                expression: DatexExpression::VariableDeclaration {
                    id: None,
                    kind: VariableKind::Const,
                    binding_mutability: BindingMutability::Immutable,
                    type_annotation: None,
                    name: "x".to_string(),
                    value: Box::new(DatexExpression::Integer(Integer::from(
                        42
                    ))),
                },
                is_terminated: true,
            },])
        );
    }

    #[test]
    fn variable_declaration_with_expression() {
        let src = "var x = 1 + 2";
        let expr = parse_unwrap(src);
        assert_eq!(
            expr,
            DatexExpression::VariableDeclaration {
                id: None,
                kind: VariableKind::Var,
                binding_mutability: BindingMutability::Mutable,
                type_annotation: None,
                name: "x".to_string(),
                value: Box::new(DatexExpression::BinaryOperation(
                    BinaryOperator::Arithmetic(ArithmeticOperator::Add),
                    Box::new(DatexExpression::Integer(Integer::from(1))),
                    Box::new(DatexExpression::Integer(Integer::from(2))),
                    None
                ))
            }
        );
    }

    #[test]
    fn variable_assignment() {
        let src = "x = 42";
        let expr = parse_unwrap(src);
        assert_eq!(
            expr,
            DatexExpression::AssignmentOperation(
                AssignmentOperator::Assign,
                None,
                "x".to_string(),
                Box::new(DatexExpression::Integer(Integer::from(42))),
            )
        );
    }

    #[test]
    fn variable_assignment_expression() {
        let src = "x = (y = 1)";
        let expr = parse_unwrap(src);
        assert_eq!(
            expr,
            DatexExpression::AssignmentOperation(
                AssignmentOperator::Assign,
                None,
                "x".to_string(),
                Box::new(DatexExpression::AssignmentOperation(
                    AssignmentOperator::Assign,
                    None,
                    "y".to_string(),
                    Box::new(DatexExpression::Integer(Integer::from(1))),
                )),
            )
        );
    }

    #[test]
    fn variable_assignment_expression_in_array() {
        let src = "[x = 1]";
        let expr = parse_unwrap(src);
        assert_eq!(
            expr,
            DatexExpression::Array(vec![DatexExpression::AssignmentOperation(
                AssignmentOperator::Assign,
                None,
                "x".to_string(),
                Box::new(DatexExpression::Integer(Integer::from(1))),
            )])
        );
    }

    #[test]
    fn apply_in_array() {
        let src = "[myFunc(1)]";
        let expr = parse_unwrap(src);
        assert_eq!(
            expr,
            DatexExpression::Array(vec![DatexExpression::ApplyChain(
                Box::new(DatexExpression::Literal("myFunc".to_string())),
                vec![ApplyOperation::FunctionCall(DatexExpression::Integer(
                    Integer::from(1)
                ))]
            ),])
        );
    }

    #[test]
    fn variant_accessor() {
        let res = parse_unwrap("integer/u8");
        assert_eq!(
            res,
            DatexExpression::BinaryOperation(
                BinaryOperator::VariantAccess,
                Box::new(DatexExpression::Literal("integer".to_string())),
                Box::new(DatexExpression::Literal("u8".to_string())),
                None
            )
        );

        let res = parse_unwrap("undeclared/u8");
        assert_eq!(
            res,
            DatexExpression::BinaryOperation(
                BinaryOperator::VariantAccess,
                Box::new(DatexExpression::Literal("undeclared".to_string())),
                Box::new(DatexExpression::Literal("u8".to_string())),
                None
            )
        );
    }

    #[test]
    fn fraction() {
        // fraction
        let res = parse_unwrap("42/3");
        assert_eq!(
            res,
            DatexExpression::Decimal(Decimal::from_string("42/3").unwrap())
        );

        let src = "1/3";
        let val = try_parse_to_value_container(src);
        assert_eq!(
            val,
            ValueContainer::from(Decimal::from_string("1/3").unwrap())
        );

        // divison
        let res = parse_unwrap("42.4/3");
        assert_eq!(
            res,
            DatexExpression::BinaryOperation(
                BinaryOperator::Arithmetic(ArithmeticOperator::Divide),
                Box::new(DatexExpression::Decimal(
                    Decimal::from_string("42.4").unwrap()
                )),
                Box::new(DatexExpression::Integer(Integer::from(3))),
                None
            )
        );

        let res = parse_unwrap("42 /3");
        assert_eq!(
            res,
            DatexExpression::BinaryOperation(
                BinaryOperator::Arithmetic(ArithmeticOperator::Divide),
                Box::new(DatexExpression::Integer(Integer::from(42))),
                Box::new(DatexExpression::Integer(Integer::from(3))),
                None
            )
        );

        let res = parse_unwrap("42/ 3");
        assert_eq!(
            res,
            DatexExpression::BinaryOperation(
                BinaryOperator::Arithmetic(ArithmeticOperator::Divide),
                Box::new(DatexExpression::Integer(Integer::from(42))),
                Box::new(DatexExpression::Integer(Integer::from(3))),
                None
            )
        );
    }

    #[test]
    fn endpoint() {
        let src = "@jonas";
        let val = try_parse_to_value_container(src);
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
        let expr = parse_unwrap(src);
        assert_eq!(
            expr,
            DatexExpression::Statements(vec![
                Statement {
                    expression: DatexExpression::VariableDeclaration {
                        id: None,
                        kind: VariableKind::Var,
                        binding_mutability: BindingMutability::Mutable,
                        name: "x".to_string(),
                        value: Box::new(DatexExpression::Integer(
                            Integer::from(42)
                        )),
                        type_annotation: None
                    },
                    is_terminated: true,
                },
                Statement {
                    expression: DatexExpression::AssignmentOperation(
                        AssignmentOperator::Assign,
                        None,
                        "x".to_string(),
                        Box::new(DatexExpression::BinaryOperation(
                            BinaryOperator::Arithmetic(
                                ArithmeticOperator::Multiply
                            ),
                            Box::new(DatexExpression::Integer(Integer::from(
                                100
                            ))),
                            Box::new(DatexExpression::Integer(Integer::from(
                                10
                            ))),
                            None
                        )),
                    ),
                    is_terminated: true,
                },
            ])
        );
    }

    #[test]
    fn placeholder() {
        let src = "?";
        let expr = parse_unwrap(src);
        assert_eq!(expr, DatexExpression::Placeholder);
    }

    #[test]
    fn integer_to_value_container() {
        let src = "123456789123456789";
        let val = try_parse_to_value_container(src);
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
        let val = try_parse_to_value_container(src);
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
        let val = try_parse_to_value_container(src);
        assert_eq!(val, ValueContainer::from("Hello, world!".to_string()));
    }

    #[test]
    fn array_to_value_container() {
        let src = "[1, 2, 3, 4.5, \"text\"]";
        let val = try_parse_to_value_container(src);
        let value_container_array: Vec<ValueContainer> = vec![
            Integer::from(1).into(),
            Integer::from(2).into(),
            Integer::from(3).into(),
            Decimal::from_string("4.5").unwrap().into(),
            "text".to_string().into(),
        ];
        assert_eq!(val, ValueContainer::from(value_container_array));
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

        let val = try_parse_to_value_container(src);
        let value_container_array: Vec<ValueContainer> = vec![
            Integer::from(1).into(),
            Integer::from(2).into(),
            Integer::from(3).into(),
            Decimal::from_string("0.5").unwrap().into(),
        ];
        let value_container_inner_object: ValueContainer =
            ValueContainer::from(Map::from(
                vec![("key".to_string(), "value".to_string().into())]
                    .into_iter()
                    .collect::<HashMap<String, ValueContainer>>(),
            ));
        let value_container_object: ValueContainer =
            ValueContainer::from(Map::from(
                vec![
                    ("name".to_string(), "Test".to_string().into()),
                    ("value".to_string(), Integer::from(42).into()),
                    ("active".to_string(), true.into()),
                    ("items".to_string(), value_container_array.into()),
                    ("nested".to_string(), value_container_inner_object),
                ]
                .into_iter()
                .collect::<HashMap<String, ValueContainer>>(),
            ));
        assert_eq!(val, value_container_object);
    }
    #[test]
    fn invalid_value_containers() {
        let src = "1 + 2";
        let expr = parse_unwrap(src);
        assert!(
            ValueContainer::try_from(&expr).is_err(),
            "Expected error when converting expression to ValueContainer"
        );

        let src = "xy";
        let expr = parse_unwrap(src);
        assert!(
            ValueContainer::try_from(&expr).is_err(),
            "Expected error when converting expression to ValueContainer"
        );

        let src = "x()";
        let expr = parse_unwrap(src);
        assert!(
            ValueContainer::try_from(&expr).is_err(),
            "Expected error when converting expression to ValueContainer"
        );
    }

    #[test]
    fn invalid_add() {
        let src = "1+2";
        let res = parse(src);
        assert!(
            res.unwrap_err().len() == 1,
            "Expected error when parsing expression"
        );
    }

    #[test]
    fn decimal_nan() {
        let src = "NaN";
        let num = parse_unwrap(src);
        assert_matches!(num, DatexExpression::Decimal(Decimal::NaN));

        let src = "nan";
        let num = parse_unwrap(src);
        assert_matches!(num, DatexExpression::Decimal(Decimal::NaN));
    }

    #[test]
    fn decimal_infinity() {
        let src = "Infinity";
        let num = parse_unwrap(src);
        assert_eq!(num, DatexExpression::Decimal(Decimal::Infinity));

        let src = "-Infinity";
        let num = parse_unwrap(src);
        assert_eq!(num, DatexExpression::Decimal(Decimal::NegInfinity));

        let src = "infinity";
        let num = parse_unwrap(src);
        assert_eq!(num, DatexExpression::Decimal(Decimal::Infinity));

        let src = "-infinity";
        let num = parse_unwrap(src);
        assert_eq!(num, DatexExpression::Decimal(Decimal::NegInfinity));
    }

    #[test]
    fn comment() {
        let src = "// This is a comment\n1 + 2";
        let expr = parse_unwrap(src);
        assert_eq!(
            expr,
            DatexExpression::BinaryOperation(
                BinaryOperator::Arithmetic(ArithmeticOperator::Add),
                Box::new(DatexExpression::Integer(Integer::from(1))),
                Box::new(DatexExpression::Integer(Integer::from(2))),
                None
            )
        );

        let src = "1 + //test\n2";
        let expr = parse_unwrap(src);
        assert_eq!(
            expr,
            DatexExpression::BinaryOperation(
                BinaryOperator::Arithmetic(ArithmeticOperator::Add),
                Box::new(DatexExpression::Integer(Integer::from(1))),
                Box::new(DatexExpression::Integer(Integer::from(2))),
                None
            )
        );
    }

    #[test]
    fn multiline_comment() {
        let src = "/* This is a\nmultiline comment */\n1 + 2";
        let expr = parse_unwrap(src);
        assert_eq!(
            expr,
            DatexExpression::BinaryOperation(
                BinaryOperator::Arithmetic(ArithmeticOperator::Add),
                Box::new(DatexExpression::Integer(Integer::from(1))),
                Box::new(DatexExpression::Integer(Integer::from(2))),
                None
            )
        );

        let src = "1 + /*test*/ 2";
        let expr = parse_unwrap(src);
        assert_eq!(
            expr,
            DatexExpression::BinaryOperation(
                BinaryOperator::Arithmetic(ArithmeticOperator::Add),
                Box::new(DatexExpression::Integer(Integer::from(1))),
                Box::new(DatexExpression::Integer(Integer::from(2))),
                None
            )
        );
    }

    #[test]
    fn shebang() {
        let src = "#!/usr/bin/env datex\n1 + 2";
        let expr = parse_unwrap(src);
        assert_eq!(
            expr,
            DatexExpression::BinaryOperation(
                BinaryOperator::Arithmetic(ArithmeticOperator::Add),
                Box::new(DatexExpression::Integer(Integer::from(1))),
                Box::new(DatexExpression::Integer(Integer::from(2))),
                None
            )
        );

        let src = "1;\n#!/usr/bin/env datex\n2";
        // syntax error
        let res = parse(src);
        assert!(
            res.is_err(),
            "Expected error when parsing expression with shebang"
        );
    }

    #[test]
    fn remote_execution() {
        let src = "a :: b";
        let expr = parse_unwrap(src);
        assert_eq!(
            expr,
            DatexExpression::RemoteExecution(
                Box::new(DatexExpression::Literal("a".to_string())),
                Box::new(DatexExpression::Literal("b".to_string()))
            )
        );
    }
    #[test]
    fn remote_execution_no_space() {
        let src = "a::b";
        let expr = parse_unwrap(src);
        assert_eq!(
            expr,
            DatexExpression::RemoteExecution(
                Box::new(DatexExpression::Literal("a".to_string())),
                Box::new(DatexExpression::Literal("b".to_string()))
            )
        );
    }

    #[test]
    fn remote_execution_complex() {
        let src = "a :: b + c * 2";
        let expr = parse_unwrap(src);
        assert_eq!(
            expr,
            DatexExpression::RemoteExecution(
                Box::new(DatexExpression::Literal("a".to_string())),
                Box::new(DatexExpression::BinaryOperation(
                    BinaryOperator::Arithmetic(ArithmeticOperator::Add),
                    Box::new(DatexExpression::Literal("b".to_string())),
                    Box::new(DatexExpression::BinaryOperation(
                        BinaryOperator::Arithmetic(
                            ArithmeticOperator::Multiply
                        ),
                        Box::new(DatexExpression::Literal("c".to_string())),
                        Box::new(DatexExpression::Integer(Integer::from(2))),
                        None
                    )),
                    None
                )),
            )
        );
    }

    #[test]
    fn remote_execution_statements() {
        let src = "a :: b; 1";
        let expr = parse_unwrap(src);
        assert_eq!(
            expr,
            DatexExpression::Statements(vec![
                Statement {
                    expression: DatexExpression::RemoteExecution(
                        Box::new(DatexExpression::Literal("a".to_string())),
                        Box::new(DatexExpression::Literal("b".to_string()))
                    ),
                    is_terminated: true,
                },
                Statement {
                    expression: DatexExpression::Integer(Integer::from(1)),
                    is_terminated: false,
                },
            ])
        );
    }

    #[test]
    fn remote_execution_inline_statements() {
        let src = "a :: (1; 2 + 3)";
        let expr = parse_unwrap(src);
        assert_eq!(
            expr,
            DatexExpression::RemoteExecution(
                Box::new(DatexExpression::Literal("a".to_string())),
                Box::new(DatexExpression::Statements(vec![
                    Statement {
                        expression: DatexExpression::Integer(Integer::from(1)),
                        is_terminated: true,
                    },
                    Statement {
                        expression: DatexExpression::BinaryOperation(
                            BinaryOperator::Arithmetic(ArithmeticOperator::Add),
                            Box::new(DatexExpression::Integer(Integer::from(
                                2
                            ))),
                            Box::new(DatexExpression::Integer(Integer::from(
                                3
                            ))),
                            None
                        ),
                        is_terminated: false,
                    },
                ])),
            )
        );
    }

    #[test]
    fn named_slot() {
        let src = "#endpoint";
        let expr = parse_unwrap(src);
        assert_eq!(
            expr,
            DatexExpression::Slot(Slot::Named("endpoint".to_string()))
        );
    }

    #[test]
    fn addressed_slot() {
        let src = "#123";
        let expr = parse_unwrap(src);
        assert_eq!(expr, DatexExpression::Slot(Slot::Addressed(123)));
    }

    #[test]
    fn variable_add_assignment() {
        let src = "x += 42";
        let expr = parse_unwrap(src);
        assert_eq!(
            expr,
            DatexExpression::AssignmentOperation(
                AssignmentOperator::AddAssign,
                None,
                "x".to_string(),
                Box::new(DatexExpression::Integer(Integer::from(42))),
            )
        );
    }

    #[test]
    fn variable_sub_assignment() {
        let src = "x -= 42";
        let expr = parse_unwrap(src);
        assert_eq!(
            expr,
            DatexExpression::AssignmentOperation(
                AssignmentOperator::SubstractAssign,
                None,
                "x".to_string(),
                Box::new(DatexExpression::Integer(Integer::from(42))),
            )
        );
    }

    #[test]
    fn variable_declaration_mut() {
        let src = "const x = &mut [1, 2, 3]";
        let expr = parse_unwrap(src);
        assert_eq!(
            expr,
            DatexExpression::VariableDeclaration {
                id: None,
                kind: VariableKind::Const,
                binding_mutability: BindingMutability::Immutable,
                name: "x".to_string(),
                type_annotation: None,
                value: Box::new(DatexExpression::RefMut(Box::new(
                    DatexExpression::Array(vec![
                        DatexExpression::Integer(Integer::from(1)),
                        DatexExpression::Integer(Integer::from(2)),
                        DatexExpression::Integer(Integer::from(3)),
                    ])
                ))),
            }
        );
    }

    #[test]
    fn variable_declaration_ref() {
        let src = "const x = &[1, 2, 3]";
        let expr = parse_unwrap(src);
        assert_eq!(
            expr,
            DatexExpression::VariableDeclaration {
                id: None,
                kind: VariableKind::Const,
                binding_mutability: BindingMutability::Immutable,
                name: "x".to_string(),
                type_annotation: None,
                value: Box::new(DatexExpression::Ref(Box::new(
                    DatexExpression::Array(vec![
                        DatexExpression::Integer(Integer::from(1)),
                        DatexExpression::Integer(Integer::from(2)),
                        DatexExpression::Integer(Integer::from(3)),
                    ])
                ))),
            }
        );
    }
    #[test]
    fn variable_declaration() {
        let src = "const x = 1";
        let expr = parse_unwrap(src);
        assert_eq!(
            expr,
            DatexExpression::VariableDeclaration {
                id: None,
                kind: VariableKind::Const,
                binding_mutability: BindingMutability::Immutable,
                name: "x".to_string(),
                type_annotation: None,
                value: Box::new(DatexExpression::Integer(Integer::from(1))),
            }
        );
    }

    #[test]
    fn negation() {
        let src = "!x";
        let expr = parse_unwrap(src);
        assert_eq!(
            expr,
            DatexExpression::UnaryOperation(
                UnaryOperator::Not,
                Box::new(DatexExpression::Literal("x".to_string()))
            )
        );

        let src = "!true";
        let expr = parse_unwrap(src);
        assert_eq!(
            expr,
            DatexExpression::UnaryOperation(
                UnaryOperator::Not,
                Box::new(DatexExpression::Boolean(true))
            )
        );

        let src = "!!(1, 2)";
        let expr = parse_unwrap(src);
        assert_matches!(
            expr,
            DatexExpression::UnaryOperation(
                UnaryOperator::Not,
                box DatexExpression::UnaryOperation(
                    UnaryOperator::Not,
                    box DatexExpression::List(_),
                ),
            )
        );
    }
}
