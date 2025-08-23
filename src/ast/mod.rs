pub mod array;
pub mod assignment_operation;
pub mod atom;
pub mod binary_operation;
pub mod chain;
pub mod comparison_operation;
pub mod decimal;
pub mod endpoint;
pub mod function;
pub mod integer;
pub mod key;
pub mod literal;
pub mod object;
pub mod text;
pub mod tuple;
pub mod unary;
pub mod unary_operation;
pub mod utils;
pub mod variable;
use chumsky::error::RichReason;
use chumsky::label::LabelError;

use crate::ast::array::*;
use crate::ast::assignment_operation::*;
use crate::ast::atom::*;
use crate::ast::binary_operation::*;
use crate::ast::chain::*;
use crate::ast::comparison_operation::*;
use crate::ast::function::*;
use crate::ast::key::*;
use crate::ast::object::*;
use crate::ast::tuple::*;
use crate::ast::unary::*;
use crate::ast::unary_operation::*;
use crate::ast::utils::*;
use crate::ast::variable::*;

use crate::values::core_values::decimal::decimal::Decimal;
use crate::values::core_values::decimal::typed_decimal::TypedDecimal;
use crate::values::core_values::endpoint::Endpoint;
use crate::values::core_values::integer::integer::Integer;
use crate::values::core_values::integer::typed_integer::TypedInteger;
use crate::values::core_values::object::Object;
use crate::values::value::Value;
use crate::values::value_container::ValueContainer;
use crate::{compiler::lexer::Token, values::core_values::array::Array};
use chumsky::error::RichPattern;
use chumsky::extra::Err;
use chumsky::prelude::*;
use chumsky::util::Maybe;
use logos::Logos;
use std::ops::Deref;
use std::{collections::HashMap, ops::Range};

#[derive(Clone, Debug, PartialEq)]
struct SpannedToken(Token);

impl Deref for SpannedToken {
    type Target = Token;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<SpannedToken> for Token {
    fn from(spanned: SpannedToken) -> Self {
        spanned.0
    }
}
impl From<Token> for SpannedToken {
    fn from(token: Token) -> Self {
        SpannedToken(token)
    }
}

impl Eq for SpannedToken {}
pub type TokenInput<'a, X = Token> = &'a [X];
pub trait DatexParserTrait<'a, T = DatexExpression, X = Token> =
    Parser<'a, TokenInput<'a, X>, T, Err<Rich<'a, X>>> + Clone + 'a
    where X: PartialEq + 'a;

pub type DatexScriptParser<'a> =
    Boxed<'a, 'a, TokenInput<'a>, DatexExpression, Err<Rich<'a, Token>>>;

#[derive(Clone, Debug, PartialEq)]
pub struct Statement {
    pub expression: DatexExpression,
    pub is_terminated: bool,
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

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ReferenceMutability {
    Mutable,
    Immutable,
    None,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Slot {
    Addressed(u32),
    Named(String),
}

pub type VariableId = usize;

#[derive(Clone, Debug, PartialEq)]
pub enum DatexExpression {
    /// Invalid expression, e.g. syntax error
    Invalid,

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
    Literal {
        name: String,
        variant: Option<String>,
    },

    /// Endpoint, e.g. @test_a or @test_b
    Endpoint(Endpoint),
    /// Array, e.g  `[1, 2, 3, "text"]`
    Array(Vec<DatexExpression>),
    /// Object, e.g {"key": "value", key2: 2}
    Object(Vec<(DatexExpression, DatexExpression)>),
    /// Tuple, e.g (1: 2, 3: 4, "xy") or without brackets: 1,2,a:3
    Tuple(Vec<TupleEntry>),
    /// One or more statements, e.g (1; 2; 3)
    Statements(Vec<Statement>),
    /// Identifier, e.g. a variable name. VariableId is always set to 0 by the ast parser.
    Variable(Option<VariableId>, String),

    /// Variable declaration, e.g. const x = 1, const mut x = 1, or var y = 2. VariableId is always set to 0 by the ast parser.
    VariableDeclaration {
        id: Option<VariableId>,
        kind: VariableKind,
        binding_mutability: BindingMutability,
        reference_mutability: ReferenceMutability,
        name: String,
        type_annotation: Option<Box<DatexExpression>>,
        value: Box<DatexExpression>,
    },

    FunctionDeclaration {
        name: String,
        parameters: Box<DatexExpression>,
        return_type: Option<Box<DatexExpression>>,
        body: Box<DatexExpression>,
    },

    /// Reference, e.g. &x
    Ref(Box<DatexExpression>),
    /// Mutable reference, e.g. &mut x
    RefMut(Box<DatexExpression>),

    /// Slot, e.g. #1, #endpoint
    Slot(Slot),
    /// Slot assignment
    SlotAssignment(Slot, Box<DatexExpression>),

    BinaryOperation(BinaryOperator, Box<DatexExpression>, Box<DatexExpression>),
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
impl TryFrom<DatexExpression> for ValueContainer {
    type Error = ();

    fn try_from(expr: DatexExpression) -> Result<Self, Self::Error> {
        Ok(match expr {
            DatexExpression::Null => ValueContainer::Value(Value::null()),
            DatexExpression::Boolean(b) => ValueContainer::from(b),
            DatexExpression::Text(s) => ValueContainer::from(s),
            DatexExpression::Decimal(d) => ValueContainer::from(d),
            DatexExpression::Integer(i) => ValueContainer::from(i),
            DatexExpression::Endpoint(e) => ValueContainer::from(e),
            DatexExpression::Array(arr) => {
                let entries = arr
                    .into_iter()
                    .map(ValueContainer::try_from)
                    .collect::<Result<Vec<ValueContainer>, ()>>()?;
                ValueContainer::from(Array::from(entries))
            }
            DatexExpression::Object(obj) => {
                let entries = obj
                    .into_iter()
                    .map(|(k, v)| {
                        let key = match k {
                            DatexExpression::Text(s) => s,
                            _ => Err(())?,
                        };
                        let value = ValueContainer::try_from(v)?;
                        Ok((key, value))
                    })
                    .collect::<Result<HashMap<String, ValueContainer>, ()>>()?;
                ValueContainer::from(Object::from(entries))
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
    let mut expression_without_tuple = Recursive::declare();

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
        .boxed();

    // expression wrapped in parentheses
    let wrapped_expression = statements
        .clone()
        .delimited_by(just(Token::LeftParen), just(Token::RightParen));

    // a valid object/tuple key
    // (1: value), "key", 1, (("x"+"y"): 123)
    let key = key(wrapped_expression.clone());

    // array
    // 1,2,3
    // [1,2,3,4,13434,(1),4,5,7,8]
    let array = array(expression_without_tuple.clone());

    // object
    let object = object(key.clone(), expression_without_tuple.clone());

    // tuple
    // Key-value pair
    let tuple = tuple(key.clone(), expression_without_tuple.clone());

    // atomic expression (e.g. 1, "text", (1 + 2), (1;2))
    let atom = atom(array.clone(), object.clone(), wrapped_expression.clone());

    let unary = unary(atom.clone());

    // apply chain: two expressions following each other directly, optionally separated with "." (property access)
    let chain = chain(
        unary.clone(),
        key.clone(),
        array.clone(),
        object.clone(),
        wrapped_expression.clone(),
        atom.clone(),
    );
    let union = binary_operation(chain);

    // FIXME WIP
    let function_declaration = function(
        statements.clone(),
        tuple.clone(),
        expression_without_tuple.clone(),
    );

    // comparison (==, !=, is, …)
    let comparison = comparison_operation(union.clone());

    // variable declarations or assignments
    let variable_assignment = variable_assignment_or_declaration(union.clone());

    expression_without_tuple.define(choice((
        variable_assignment,
        function_declaration,
        comparison,
    )));

    // expression :: expression
    let remote_execution = expression_without_tuple
        .clone()
        .then_ignore(just(Token::DoubleColon).padded_by(whitespace()))
        .then(expression_without_tuple.clone())
        .map(|(endpoint, expr)| {
            DatexExpression::RemoteExecution(Box::new(endpoint), Box::new(expr))
        });

    expression.define(
        choice((
            remote_execution,
            tuple.clone(),
            expression_without_tuple.clone(),
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

// #[derive(Debug, Clone)]
pub enum ParserError {
    UnexpectedToken(DatexRich<'static, Token>),
    InvalidToken(Range<usize>),
}
use ariadne::{Color, Label, Report, ReportKind, Source};
use std::fmt::Debug;

impl Debug for ParserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParserError::UnexpectedToken(rich) => {
                write!(f, "Unexpected token: {:?}", rich.span)
            }
            ParserError::InvalidToken(range) => {
                write!(f, "Invalid token at range: {:?}", range)
            }
        }
    }
}

fn report_from_rich(
    error: &DatexRich<'static, Token>,
    src_id: &str,
    src: &str,
) {
    let msg = if let RichReason::Custom(msg) = error.reason() {
        msg.clone()
    } else {
        let mut normal_items = Vec::new();
        let mut has_something_else = false;

        for expected in error.expected() {
            match expected {
                RichPattern::Token(token) => {
                    normal_items.push(match token {
                        Maybe::Ref(token) => token.to_string().to_lowercase(),
                        Maybe::Val(token) => token.to_string().to_lowercase(),
                    });
                }
                RichPattern::Label(label) => {
                    normal_items.push(format!("label '{}'", label));
                }
                RichPattern::Identifier(id) => {
                    normal_items.push(format!("identifier '{}'", id));
                }
                RichPattern::Any => {
                    normal_items.push("anything".to_string());
                }
                RichPattern::EndOfInput => {
                    normal_items.push("end of input".to_string());
                }
                RichPattern::SomethingElse => {
                    has_something_else = true;
                }
            }
        }

        // Build final list, putting `something else` at the end if needed
        if has_something_else {
            normal_items.push("something else".to_string());
        }

        // Format nicely with commas and "or"
        let expected_str = match normal_items.len() {
            0 => "something else".to_string(),
            1 => normal_items[0].clone(),
            2 => format!("{} or {}", normal_items[0], normal_items[1]),
            _ => {
                let last = normal_items.pop().unwrap();
                format!("{}, or {}", normal_items.join(", "), last)
            }
        };

        format!(
            "Unexpected {}, expected {}.",
            error
                .found()
                .map(|c| format!("token {}", c.to_string().to_lowercase()))
                .unwrap_or_else(|| "end of input".to_string()),
            expected_str
        )
    };
    let report =
        Report::build(ReportKind::Error, (src_id, error.span().clone()))
            .with_code("Unexpected Token")
            .with_message(msg)
            .with_note("Please check the syntax and try again.")
            .with_label(
                Label::new((src_id, error.span().clone()))
                    .with_message(match error.reason() {
                        RichReason::Custom(msg) => msg.clone(),
                        _ => format!(
                            "Unexpected {}",
                            error
                                .found()
                                .map(|c| format!("token {}", c))
                                .unwrap_or_else(|| "end of input".to_string())
                        ),
                    })
                    .with_color(Color::Red),
            );
    report.finish().eprint((src_id, Source::from(src))).unwrap();
}

pub struct ErrorCollector {
    pub errors: Vec<ParserError>,
    pub src: String,
}
impl ErrorCollector {
    pub fn new(src: String) -> Self {
        Self {
            errors: Vec::new(),
            src,
        }
    }
    pub fn new_with_errors(src: String, errors: Vec<ParserError>) -> Self {
        Self { errors, src }
    }

    pub fn add_error(&mut self, error: ParserError) {
        self.errors.push(error);
    }

    pub fn print(&self) {
        let src = &self.src;
        for error in &self.errors {
            match error {
                ParserError::UnexpectedToken(rich) => {
                    report_from_rich(rich, "datex", src);
                }
                ParserError::InvalidToken(range) => {
                    println!("Invalid token error: {:?}", range);
                }
            }
        }
    }
}

impl From<Range<usize>> for ParserError {
    fn from(range: Range<usize>) -> Self {
        ParserError::InvalidToken(range)
    }
}

pub struct DatexRich<'a, T> {
    span: Range<usize>,
    rich: Rich<'a, T>,
}
impl Deref for DatexRich<'static, Token> {
    type Target = Rich<'static, Token>;

    fn deref(&self) -> &Self::Target {
        &self.rich
    }
}
impl<'a, T> DatexRich<'a, T> {
    pub fn new(span: Range<usize>, rich: Rich<'a, T>) -> Self {
        Self { span, rich }
    }

    pub fn span(&self) -> &Range<usize> {
        &self.span
    }

    pub fn into_owned<'b>(self) -> DatexRich<'b, T>
    where
        T: Clone,
    {
        DatexRich::new(self.span().clone(), self.rich.into_owned())
    }
}

impl<'a, T> From<Rich<'a, T>> for DatexRich<'a, T> {
    fn from(rich: Rich<'a, T>) -> Self {
        let span = rich.span().into_range();
        DatexRich::new(span, rich)
    }
}

pub fn parse(mut src: &str) -> Result<DatexExpression, Vec<ParserError>> {
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
                .map_err(|_| ParserError::InvalidToken(span))
        })
        .collect::<Result<_, _>>()
        .map_err(|e| vec![e])?;

    let (tokens, spans): (Vec<_>, Vec<_>) = tokens_spanned.into_iter().unzip();
    let parser = create_parser::<'_, Token>();
    parser.parse(&tokens).into_result().map_err(|err| {
        err.into_iter()
            .map(|e| {
                let owned_rich = e.to_owned().clone();
                let range = owned_rich.span().into_range();
                let span = spans.get(range.start).unwrap();
                let rich = DatexRich::new(span.clone(), owned_rich);
                ParserError::UnexpectedToken(rich.into_owned())
            })
            .collect()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{assert_matches::assert_matches, str::FromStr};

    fn parse_unwrap(src: &str) -> DatexExpression {
        let res = parse(src);
        if let Err(errors) = res {
            ErrorCollector::new_with_errors(src.to_string(), errors).print();
            panic!("Parsing errors found");
        }
        res.unwrap()
    }

    fn try_parse_to_value_container(src: &str) -> ValueContainer {
        let expr = parse_unwrap(src);
        ValueContainer::try_from(expr).unwrap_or_else(|_| {
            panic!("Failed to convert expression to ValueContainer")
        })
    }

    #[test]
    fn test_json() {
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
            DatexExpression::Object(vec![
                (
                    DatexExpression::Text("name".to_string()),
                    DatexExpression::Text("Test".to_string())
                ),
                (
                    DatexExpression::Text("value".to_string()),
                    DatexExpression::Integer(Integer::from(42))
                ),
                (
                    DatexExpression::Text("active".to_string()),
                    DatexExpression::Boolean(true)
                ),
                (
                    DatexExpression::Text("items".to_string()),
                    DatexExpression::Array(vec![
                        DatexExpression::Integer(Integer::from(1)),
                        DatexExpression::Integer(Integer::from(2)),
                        DatexExpression::Integer(Integer::from(3)),
                        DatexExpression::Decimal(Decimal::from_string("0.5"))
                    ])
                ),
                (
                    DatexExpression::Text("nested".to_string()),
                    DatexExpression::Object(
                        vec![(
                            DatexExpression::Text("key".to_string()),
                            DatexExpression::Text("value".to_string())
                        )]
                        .into_iter()
                        .collect()
                    )
                ),
            ])
        );
    }

    // WIP
    #[test]
    fn test_parse_error() {
        let src = r#"
        var x = 52; var y = ; 
        var y = 5
        "#;
        let res = parse_unwrap(src);
        // assert!(res.is_err());
        // println!("{:?}", res.unwrap_err());
    }

    #[test]
    fn test_function_simple() {
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
                parameters: Box::new(DatexExpression::Tuple(vec![])),
                return_type: None,
                body: Box::new(DatexExpression::Integer(Integer::from(42))),
            }
        );
    }

    #[test]
    fn test_function_with_params() {
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
                parameters: Box::new(DatexExpression::Tuple(vec![
                    TupleEntry::KeyValue(
                        DatexExpression::Text("x".to_string()),
                        DatexExpression::Literal {
                            name: "integer".to_owned(),
                            variant: None
                        }
                    )
                ])),
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
                parameters: Box::new(DatexExpression::Tuple(vec![
                    TupleEntry::KeyValue(
                        DatexExpression::Text("x".to_string()),
                        DatexExpression::Literal {
                            name: "integer".to_owned(),
                            variant: None
                        }
                    ),
                    TupleEntry::KeyValue(
                        DatexExpression::Text("y".to_string()),
                        DatexExpression::Literal {
                            name: "integer".to_owned(),
                            variant: None
                        }
                    )
                ])),
                return_type: None,
                body: Box::new(DatexExpression::Statements(vec![Statement {
                    expression: DatexExpression::BinaryOperation(
                        BinaryOperator::Add,
                        Box::new(DatexExpression::Integer(Integer::from(1))),
                        Box::new(DatexExpression::Integer(Integer::from(2)))
                    ),
                    is_terminated: true
                }])),
            }
        );
    }

    // FIXME WIP
    #[test]
    #[ignore = "WIP"]
    fn test_function_with_return_type() {
        let src = r#"
            function myFunction(x: integer) -> integer (
                42
            );
        "#;
        let val = parse_unwrap(src);
        assert_eq!(
            val,
            DatexExpression::FunctionDeclaration {
                name: "myFunction".to_string(),
                parameters: Box::new(DatexExpression::Tuple(vec![
                    TupleEntry::KeyValue(
                        DatexExpression::Text("x".to_string()),
                        DatexExpression::Literal {
                            name: "integer".to_owned(),
                            variant: None
                        }
                    )
                ])),
                return_type: Some(Box::new(DatexExpression::Literal {
                    name: "integer".to_owned(),
                    variant: None
                })),
                body: Box::new(DatexExpression::Integer(Integer::from(42))),
            }
        );
    }

    #[test]
    fn test_type_var_declaration() {
        let src = "var x: 5 = 42";
        let val = parse_unwrap(src);
        assert_eq!(
            val,
            DatexExpression::VariableDeclaration {
                id: None,
                kind: VariableKind::Var,
                binding_mutability: BindingMutability::Mutable,
                reference_mutability: ReferenceMutability::None,
                type_annotation: Some(Box::new(DatexExpression::Integer(
                    Integer::from(5)
                ))),
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
                reference_mutability: ReferenceMutability::None,
                type_annotation: Some(Box::new(DatexExpression::Literal {
                    name: "integer".to_owned(),
                    variant: Some("u8".to_owned())
                })),
                name: "x".to_string(),
                value: Box::new(DatexExpression::Integer(Integer::from(42)))
            }
        );
    }

    #[test]
    fn test_intersection() {
        let src = "5 & 6";
        let val = parse_unwrap(src);
        assert_eq!(
            val,
            DatexExpression::BinaryOperation(
                BinaryOperator::Intersection,
                Box::new(DatexExpression::Integer(Integer::from(5))),
                Box::new(DatexExpression::Integer(Integer::from(6)))
            )
        );

        let src = "(integer/u8 & 6) & 2";
        let val = parse_unwrap(src);
        assert_eq!(
            val,
            DatexExpression::BinaryOperation(
                BinaryOperator::Intersection,
                Box::new(DatexExpression::BinaryOperation(
                    BinaryOperator::Intersection,
                    Box::new(DatexExpression::Literal {
                        name: "integer".to_owned(),
                        variant: Some("u8".to_owned())
                    }),
                    Box::new(DatexExpression::Integer(Integer::from(6)))
                )),
                Box::new(DatexExpression::Integer(Integer::from(2)))
            )
        );
    }

    #[test]
    fn test_union() {
        let src = "5 | 6";
        let val = parse_unwrap(src);
        assert_eq!(
            val,
            DatexExpression::BinaryOperation(
                BinaryOperator::Union,
                Box::new(DatexExpression::Integer(Integer::from(5))),
                Box::new(DatexExpression::Integer(Integer::from(6)))
            )
        );

        let src = "(integer/u8 | 6) | 2";
        let val = parse_unwrap(src);
        assert_eq!(
            val,
            DatexExpression::BinaryOperation(
                BinaryOperator::Union,
                Box::new(DatexExpression::BinaryOperation(
                    BinaryOperator::Union,
                    Box::new(DatexExpression::Literal {
                        name: "integer".to_owned(),
                        variant: Some("u8".to_owned())
                    }),
                    Box::new(DatexExpression::Integer(Integer::from(6)))
                )),
                Box::new(DatexExpression::Integer(Integer::from(2)))
            )
        );
    }

    #[test]
    fn test_binary_operator_precedence() {
        let src = "1 + 2 * 3";
        let val = parse_unwrap(src);
        assert_eq!(
            val,
            DatexExpression::BinaryOperation(
                BinaryOperator::Add,
                Box::new(DatexExpression::Integer(Integer::from(1))),
                Box::new(DatexExpression::BinaryOperation(
                    BinaryOperator::Multiply,
                    Box::new(DatexExpression::Integer(Integer::from(2))),
                    Box::new(DatexExpression::Integer(Integer::from(3)))
                ))
            )
        );

        let src = "1 + 2 & 3";
        let val = parse_unwrap(src);
        assert_eq!(
            val,
            DatexExpression::BinaryOperation(
                BinaryOperator::Intersection,
                Box::new(DatexExpression::BinaryOperation(
                    BinaryOperator::Add,
                    Box::new(DatexExpression::Integer(Integer::from(1))),
                    Box::new(DatexExpression::Integer(Integer::from(2)))
                )),
                Box::new(DatexExpression::Integer(Integer::from(3)))
            )
        );

        let src = "1 + 2 | 3";
        let val = parse_unwrap(src);
        assert_eq!(
            val,
            DatexExpression::BinaryOperation(
                BinaryOperator::Union,
                Box::new(DatexExpression::BinaryOperation(
                    BinaryOperator::Add,
                    Box::new(DatexExpression::Integer(Integer::from(1))),
                    Box::new(DatexExpression::Integer(Integer::from(2)))
                )),
                Box::new(DatexExpression::Integer(Integer::from(3)))
            )
        );
    }

    #[test]
    fn test_var_declaration_with_type_simple() {
        let src = "var x: integer = 42";
        let val = parse_unwrap(src);
        assert_eq!(
            val,
            DatexExpression::VariableDeclaration {
                id: None,
                kind: VariableKind::Var,
                binding_mutability: BindingMutability::Mutable,
                reference_mutability: ReferenceMutability::None,
                type_annotation: Some(Box::new(DatexExpression::Literal {
                    name: "integer".to_string(),
                    variant: None
                })),
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
                reference_mutability: ReferenceMutability::None,
                type_annotation: Some(Box::new(DatexExpression::Literal {
                    name: "User".to_string(),
                    variant: None
                })),
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
                reference_mutability: ReferenceMutability::None,
                type_annotation: Some(Box::new(DatexExpression::Literal {
                    name: "integer".to_string(),
                    variant: Some("u8".to_string())
                })),
                name: "x".to_string(),
                value: Box::new(DatexExpression::Integer(Integer::from(42)))
            }
        );
    }

    #[test]
    fn test_var_declaration_with_type_union() {
        let src = "var x: 5 | 6 = 42";
        let val = parse_unwrap(src);
        assert_eq!(
            val,
            DatexExpression::VariableDeclaration {
                id: None,
                kind: VariableKind::Var,
                binding_mutability: BindingMutability::Mutable,
                reference_mutability: ReferenceMutability::None,
                type_annotation: Some(Box::new(
                    DatexExpression::BinaryOperation(
                        BinaryOperator::Union,
                        Box::new(DatexExpression::Integer(Integer::from(5))),
                        Box::new(DatexExpression::Integer(Integer::from(6)))
                    )
                )),
                name: "x".to_string(),
                value: Box::new(DatexExpression::Integer(Integer::from(42)))
            }
        );
    }

    #[test]
    fn test_var_declaration_with_type_intersection() {
        let src = "var x: 5 & 6 = 42";
        let val = parse_unwrap(src);
        assert_eq!(
            val,
            DatexExpression::VariableDeclaration {
                id: None,
                kind: VariableKind::Var,
                binding_mutability: BindingMutability::Mutable,
                reference_mutability: ReferenceMutability::None,
                type_annotation: Some(Box::new(
                    DatexExpression::BinaryOperation(
                        BinaryOperator::Intersection,
                        Box::new(DatexExpression::Integer(Integer::from(5))),
                        Box::new(DatexExpression::Integer(Integer::from(6)))
                    )
                )),
                name: "x".to_string(),
                value: Box::new(DatexExpression::Integer(Integer::from(42)))
            }
        );
    }

    #[test]
    #[ignore = "TBD"]
    fn test_type_var_declaration_array() {
        // FIXME what would be a syntax for array declarations that doesn't collide with apply chains
        // myfunc [] // function call with empty array
        // myfunc [5] // function call with array
        // var x: 5[] // special declaration only valid after colon and only when no space used?
        let src = "var x: integer[] = 42";
        let val = parse_unwrap(src);
        assert_eq!(
            val,
            DatexExpression::VariableDeclaration {
                id: None,
                kind: VariableKind::Var,
                binding_mutability: BindingMutability::Mutable,
                reference_mutability: ReferenceMutability::None,
                type_annotation: Some(Box::new(DatexExpression::ApplyChain(
                    Box::new(DatexExpression::Literal {
                        name: "integer".to_string(),
                        variant: None
                    }),
                    vec![ApplyOperation::ArrayType]
                ))),
                name: "x".to_string(),
                value: Box::new(DatexExpression::Integer(Integer::from(42)))
            }
        );
    }

    #[test]
    fn test_equal_operators() {
        let src = "3 == 1 + 2";
        let val = parse_unwrap(src);
        assert_eq!(
            val,
            DatexExpression::ComparisonOperation(
                ComparisonOperator::StructuralEqual,
                Box::new(DatexExpression::Integer(Integer::from(3))),
                Box::new(DatexExpression::BinaryOperation(
                    BinaryOperator::Add,
                    Box::new(DatexExpression::Integer(Integer::from(1))),
                    Box::new(DatexExpression::Integer(Integer::from(2)))
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
                    BinaryOperator::Add,
                    Box::new(DatexExpression::Integer(Integer::from(1))),
                    Box::new(DatexExpression::Integer(Integer::from(2)))
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
                    BinaryOperator::Add,
                    Box::new(DatexExpression::Integer(Integer::from(1))),
                    Box::new(DatexExpression::Integer(Integer::from(2)))
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
                    BinaryOperator::Add,
                    Box::new(DatexExpression::Integer(Integer::from(1))),
                    Box::new(DatexExpression::Integer(Integer::from(2)))
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
                    BinaryOperator::Add,
                    Box::new(DatexExpression::Integer(Integer::from(1))),
                    Box::new(DatexExpression::Integer(Integer::from(2)))
                ))
            )
        );
    }

    #[test]
    fn test_null() {
        let src = "null";
        let val = parse_unwrap(src);
        assert_eq!(val, DatexExpression::Null);
    }

    #[test]
    fn test_boolean() {
        let src_true = "true";
        let val_true = parse_unwrap(src_true);
        assert_eq!(val_true, DatexExpression::Boolean(true));

        let src_false = "false";
        let val_false = parse_unwrap(src_false);
        assert_eq!(val_false, DatexExpression::Boolean(false));
    }

    #[test]
    fn test_integer() {
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
    fn test_negative_integer() {
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
    fn test_integer_with_underscores() {
        let src = "123_456";
        let num = parse_unwrap(src);
        assert_eq!(
            num,
            DatexExpression::Integer(Integer::from_string("123456").unwrap())
        );
    }

    #[test]
    fn test_hex_integer() {
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
    fn test_octal_integer() {
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
    fn test_binary_integer() {
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
    fn test_integer_with_exponent() {
        let src = "2e10";
        let num = parse_unwrap(src);
        assert_eq!(
            num,
            DatexExpression::Decimal(Decimal::from_string("20000000000"))
        );
    }

    #[test]
    fn test_decimal() {
        let src = "123.456789123456";
        let num = parse_unwrap(src);
        assert_eq!(
            num,
            DatexExpression::Decimal(Decimal::from_string("123.456789123456"))
        );
    }

    #[test]
    fn test_decimal_with_separator() {
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
                DatexExpression::Decimal(Decimal::from_string(expected_str)),
                "Failed to parse: {src}"
            );
        }
    }

    #[test]
    fn test_negative_decimal() {
        let src = "-123.4";
        let num = parse_unwrap(src);
        assert_eq!(
            num,
            DatexExpression::Decimal(Decimal::from_string("-123.4"))
        );
    }

    #[test]
    fn test_decimal_with_exponent() {
        let src = "1.23456789123456e2";
        let num = parse_unwrap(src);
        assert_eq!(
            num,
            DatexExpression::Decimal(Decimal::from_string("123.456789123456"))
        );
    }

    #[test]
    fn test_decimal_with_negative_exponent() {
        let src = "1.23456789123456e-2";
        let num = parse_unwrap(src);
        assert_eq!(
            num,
            DatexExpression::Decimal(Decimal::from_string(
                "0.0123456789123456"
            ))
        );
    }

    #[test]
    fn test_decimal_with_positive_exponent() {
        let src = "1.23456789123456E+2";
        let num = parse_unwrap(src);
        assert_eq!(
            num,
            DatexExpression::Decimal(Decimal::from_string("123.456789123456"))
        );
    }

    #[test]
    fn test_decimal_with_trailing_point() {
        let src = "123.";
        let num = parse_unwrap(src);
        assert_eq!(
            num,
            DatexExpression::Decimal(Decimal::from_string("123.0"))
        );
    }

    #[test]
    fn test_decimal_with_leading_point() {
        let src = ".456789123456";
        let num = parse_unwrap(src);
        assert_eq!(
            num,
            DatexExpression::Decimal(Decimal::from_string("0.456789123456"))
        );

        let src = ".423e-2";
        let num = parse_unwrap(src);
        assert_eq!(
            num,
            DatexExpression::Decimal(Decimal::from_string("0.00423"))
        );
    }

    #[test]
    fn test_text_double_quotes() {
        let src = r#""Hello, world!""#;
        let text = parse_unwrap(src);
        assert_eq!(text, DatexExpression::Text("Hello, world!".to_string()));
    }

    #[test]
    fn test_text_single_quotes() {
        let src = r#"'Hello, world!'"#;
        let text = parse_unwrap(src);
        assert_eq!(text, DatexExpression::Text("Hello, world!".to_string()));
    }

    #[test]
    fn test_text_escape_sequences() {
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
    fn test_text_escape_sequences_2() {
        let src =
            r#""\u0048\u0065\u006C\u006C\u006F, \u2764\uFE0F, \uD83D\uDE00""#;
        let text = parse_unwrap(src);
        assert_eq!(text, DatexExpression::Text("Hello, ❤️, 😀".to_string()));
    }

    #[test]
    fn test_text_nested_escape_sequences() {
        let src = r#""\\\\""#;
        let text = parse_unwrap(src);
        assert_eq!(text, DatexExpression::Text("\\\\".to_string()));
    }

    #[test]
    fn test_text_nested_escape_sequences_2() {
        let src = r#""\\\"""#;
        let text = parse_unwrap(src);
        assert_eq!(text, DatexExpression::Text("\\\"".to_string()));
    }

    #[test]
    fn test_empty_array() {
        let src = "[]";
        let arr = parse_unwrap(src);
        assert_eq!(arr, DatexExpression::Array(vec![]));
    }

    #[test]
    fn test_array_with_values() {
        let src = "[1, 2, 3, 4.5, \"text\"]";
        let arr = parse_unwrap(src);

        assert_eq!(
            arr,
            DatexExpression::Array(vec![
                DatexExpression::Integer(Integer::from(1)),
                DatexExpression::Integer(Integer::from(2)),
                DatexExpression::Integer(Integer::from(3)),
                DatexExpression::Decimal(Decimal::from_string("4.5")),
                DatexExpression::Text("text".to_string()),
            ])
        );
    }

    #[test]
    fn test_empty_object() {
        let src = "{}";
        let obj = parse_unwrap(src);

        assert_eq!(obj, DatexExpression::Object(vec![]));
    }

    #[test]
    fn test_tuple() {
        let src = "1,2";
        let tuple = parse_unwrap(src);

        assert_eq!(
            tuple,
            DatexExpression::Tuple(vec![
                TupleEntry::Value(DatexExpression::Integer(Integer::from(1))),
                TupleEntry::Value(DatexExpression::Integer(Integer::from(2))),
            ])
        );
    }

    #[test]
    fn test_scoped_tuple() {
        let src = "(1, 2)";
        let tuple = parse_unwrap(src);

        assert_eq!(
            tuple,
            DatexExpression::Tuple(vec![
                TupleEntry::Value(DatexExpression::Integer(Integer::from(1))),
                TupleEntry::Value(DatexExpression::Integer(Integer::from(2))),
            ])
        );
    }

    #[test]
    fn test_keyed_tuple() {
        let src = "1: 2, 3: 4, xy:2, 'a b c': 'd'";
        let tuple = parse_unwrap(src);

        assert_eq!(
            tuple,
            DatexExpression::Tuple(vec![
                TupleEntry::KeyValue(
                    DatexExpression::Integer(Integer::from(1)),
                    DatexExpression::Integer(Integer::from(2))
                ),
                TupleEntry::KeyValue(
                    DatexExpression::Integer(Integer::from(3)),
                    DatexExpression::Integer(Integer::from(4))
                ),
                TupleEntry::KeyValue(
                    DatexExpression::Text("xy".to_string()),
                    DatexExpression::Integer(Integer::from(2))
                ),
                TupleEntry::KeyValue(
                    DatexExpression::Text("a b c".to_string()),
                    DatexExpression::Text("d".to_string())
                ),
            ])
        );
    }

    #[test]
    fn test_tuple_array() {
        let src = "[(1,2),3,(4,)]";
        let arr = parse_unwrap(src);

        assert_eq!(
            arr,
            DatexExpression::Array(vec![
                DatexExpression::Tuple(vec![
                    TupleEntry::Value(DatexExpression::Integer(Integer::from(
                        1
                    ))),
                    TupleEntry::Value(DatexExpression::Integer(Integer::from(
                        2
                    ))),
                ]),
                DatexExpression::Integer(Integer::from(3)),
                DatexExpression::Tuple(vec![TupleEntry::Value(
                    DatexExpression::Integer(Integer::from(4))
                ),]),
            ])
        );
    }

    #[test]
    fn test_single_value_tuple() {
        let src = "1,";
        let tuple = parse_unwrap(src);

        assert_eq!(
            tuple,
            DatexExpression::Tuple(vec![TupleEntry::Value(
                DatexExpression::Integer(Integer::from(1))
            ),])
        );
    }

    #[test]
    fn test_single_key_value_tuple() {
        let src = "x: 1";
        let tuple = parse_unwrap(src);
        assert_eq!(
            tuple,
            DatexExpression::Tuple(vec![TupleEntry::KeyValue(
                DatexExpression::Text("x".to_string()),
                DatexExpression::Integer(Integer::from(1))
            ),])
        );
    }

    #[test]
    fn test_scoped_atom() {
        let src = "(1)";
        let atom = parse_unwrap(src);
        assert_eq!(atom, DatexExpression::Integer(Integer::from(1)));
    }

    #[test]
    fn test_scoped_array() {
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
    fn test_object_with_key_value_pairs() {
        let src = r#"{"key1": "value1", "key2": 42, "key3": true}"#;
        let obj = parse_unwrap(src);

        assert_eq!(
            obj,
            DatexExpression::Object(vec![
                (
                    DatexExpression::Text("key1".to_string()),
                    DatexExpression::Text("value1".to_string())
                ),
                (
                    DatexExpression::Text("key2".to_string()),
                    DatexExpression::Integer(Integer::from(42))
                ),
                (
                    DatexExpression::Text("key3".to_string()),
                    DatexExpression::Boolean(true)
                ),
            ])
        );
    }

    #[test]
    fn test_dynamic_object_keys() {
        let src = r#"{(1): "value1", (2): 42, (3): true}"#;
        let obj = parse_unwrap(src);
        assert_eq!(
            obj,
            DatexExpression::Object(vec![
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
    fn test_dynamic_tuple_keys() {
        let src = "(1): 1, ([]): 2";
        let tuple = parse_unwrap(src);

        assert_eq!(
            tuple,
            DatexExpression::Tuple(vec![
                TupleEntry::KeyValue(
                    DatexExpression::Integer(Integer::from(1)),
                    DatexExpression::Integer(Integer::from(1))
                ),
                TupleEntry::KeyValue(
                    DatexExpression::Array(vec![]),
                    DatexExpression::Integer(Integer::from(2))
                ),
            ])
        );
    }

    #[test]
    fn test_add() {
        // Test with escaped characters in text
        let src = "1 + 2";
        let expr = parse_unwrap(src);
        assert_eq!(
            expr,
            DatexExpression::BinaryOperation(
                BinaryOperator::Add,
                Box::new(DatexExpression::Integer(Integer::from(1))),
                Box::new(DatexExpression::Integer(Integer::from(2))),
            )
        );
    }

    #[test]
    fn test_add_complex_values() {
        // Test with escaped characters in text
        let src = "[] + x + (1 + 2)";
        let expr = parse_unwrap(src);
        assert_eq!(
            expr,
            DatexExpression::BinaryOperation(
                BinaryOperator::Add,
                Box::new(DatexExpression::BinaryOperation(
                    BinaryOperator::Add,
                    Box::new(DatexExpression::Array(vec![])),
                    Box::new(DatexExpression::Literal {
                        name: "x".to_string(),
                        variant: None,
                    }),
                )),
                Box::new(DatexExpression::BinaryOperation(
                    BinaryOperator::Add,
                    Box::new(DatexExpression::Integer(Integer::from(1))),
                    Box::new(DatexExpression::Integer(Integer::from(2))),
                )),
            )
        );
    }

    #[test]
    fn test_subtract() {
        let src = "5 - 3";
        let expr = parse_unwrap(src);
        assert_eq!(
            expr,
            DatexExpression::BinaryOperation(
                BinaryOperator::Subtract,
                Box::new(DatexExpression::Integer(Integer::from(5))),
                Box::new(DatexExpression::Integer(Integer::from(3))),
            )
        );
    }

    #[test]
    fn test_multiply() {
        let src = "4 * 2";
        let expr = parse_unwrap(src);
        assert_eq!(
            expr,
            DatexExpression::BinaryOperation(
                BinaryOperator::Multiply,
                Box::new(DatexExpression::Integer(Integer::from(4))),
                Box::new(DatexExpression::Integer(Integer::from(2))),
            )
        );
    }

    #[test]
    fn test_divide() {
        let src = "8 / 2";
        let expr = parse_unwrap(src);
        assert_eq!(
            expr,
            DatexExpression::BinaryOperation(
                BinaryOperator::Divide,
                Box::new(DatexExpression::Integer(Integer::from(8))),
                Box::new(DatexExpression::Integer(Integer::from(2))),
            )
        );
    }

    #[test]
    fn test_complex_calculation() {
        let src = "1 + 2 * 3 + 4";
        let expr = parse_unwrap(src);
        assert_eq!(
            expr,
            DatexExpression::BinaryOperation(
                BinaryOperator::Add,
                Box::new(DatexExpression::BinaryOperation(
                    BinaryOperator::Add,
                    Box::new(DatexExpression::Integer(Integer::from(1))),
                    Box::new(DatexExpression::BinaryOperation(
                        BinaryOperator::Multiply,
                        Box::new(DatexExpression::Integer(Integer::from(2))),
                        Box::new(DatexExpression::Integer(Integer::from(3))),
                    )),
                )),
                Box::new(DatexExpression::Integer(Integer::from(4))),
            )
        );
    }

    #[test]
    fn test_nested_addition() {
        let src = "1 + (2 + 3)";
        let expr = parse_unwrap(src);
        assert_eq!(
            expr,
            DatexExpression::BinaryOperation(
                BinaryOperator::Add,
                Box::new(DatexExpression::Integer(Integer::from(1))),
                Box::new(DatexExpression::BinaryOperation(
                    BinaryOperator::Add,
                    Box::new(DatexExpression::Integer(Integer::from(2))),
                    Box::new(DatexExpression::Integer(Integer::from(3))),
                )),
            )
        );
    }

    #[test]
    fn test_add_statements_1() {
        // Test with escaped characters in text
        let src = "1 + (2;3)";
        let expr = parse_unwrap(src);
        assert_eq!(
            expr,
            DatexExpression::BinaryOperation(
                BinaryOperator::Add,
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
            )
        );
    }

    #[test]
    fn test_add_statements_2() {
        // Test with escaped characters in text
        let src = "(1;2) + 3";
        let expr = parse_unwrap(src);
        assert_eq!(
            expr,
            DatexExpression::BinaryOperation(
                BinaryOperator::Add,
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
            )
        );
    }

    #[test]
    fn test_nested_expressions() {
        let src = "[1 + 2]";
        let expr = parse_unwrap(src);
        assert_eq!(
            expr,
            DatexExpression::Array(vec![DatexExpression::BinaryOperation(
                BinaryOperator::Add,
                Box::new(DatexExpression::Integer(Integer::from(1))),
                Box::new(DatexExpression::Integer(Integer::from(2))),
            ),])
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
            DatexExpression::Object(vec![(
                DatexExpression::Text("key".to_string()),
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
    fn test_single_statement() {
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
    fn test_empty_statement() {
        let src = ";";
        let expr = parse_unwrap(src);
        assert_eq!(expr, DatexExpression::Statements(vec![]));
    }

    #[test]
    fn test_empty_statement_multiple() {
        let src = ";;;";
        let expr = parse_unwrap(src);
        assert_eq!(expr, DatexExpression::Statements(vec![]));
    }

    #[test]
    fn test_variable_expression() {
        let src = "myVar";
        let expr = parse_unwrap(src);
        assert_eq!(
            expr,
            DatexExpression::Literal {
                name: "myVar".to_string(),
                variant: None,
            }
        );
    }

    #[test]
    fn test_variable_expression_with_operations() {
        let src = "myVar + 1";
        let expr = parse_unwrap(src);
        assert_eq!(
            expr,
            DatexExpression::BinaryOperation(
                BinaryOperator::Add,
                Box::new(DatexExpression::Literal {
                    name: "myVar".to_string(),
                    variant: None,
                }),
                Box::new(DatexExpression::Integer(Integer::from(1))),
            )
        );
    }

    #[test]
    fn test_apply_expression() {
        let src = "myFunc(1, 2, 3)";
        let expr = parse_unwrap(src);
        assert_eq!(
            expr,
            DatexExpression::ApplyChain(
                Box::new(DatexExpression::Literal {
                    name: "myFunc".to_string(),
                    variant: None,
                }),
                vec![ApplyOperation::FunctionCall(DatexExpression::Tuple(
                    vec![
                        TupleEntry::Value(DatexExpression::Integer(
                            Integer::from(1)
                        )),
                        TupleEntry::Value(DatexExpression::Integer(
                            Integer::from(2)
                        )),
                        TupleEntry::Value(DatexExpression::Integer(
                            Integer::from(3)
                        )),
                    ]
                ),)],
            )
        );
    }

    #[test]
    fn test_apply_empty() {
        let src = "myFunc()";
        let expr = parse_unwrap(src);
        assert_eq!(
            expr,
            DatexExpression::ApplyChain(
                Box::new(DatexExpression::Literal {
                    name: "myFunc".to_string(),
                    variant: None,
                }),
                vec![ApplyOperation::FunctionCall(
                    DatexExpression::Statements(vec![])
                )],
            )
        );
    }

    #[test]
    fn test_apply_multiple() {
        let src = "myFunc(1)(2, 3)";
        let expr = parse_unwrap(src);
        assert_eq!(
            expr,
            DatexExpression::ApplyChain(
                Box::new(DatexExpression::Literal {
                    name: "myFunc".to_string(),
                    variant: None,
                }),
                vec![
                    ApplyOperation::FunctionCall(DatexExpression::Integer(
                        Integer::from(1)
                    ),),
                    ApplyOperation::FunctionCall(DatexExpression::Tuple(vec![
                        TupleEntry::Value(DatexExpression::Integer(
                            Integer::from(2)
                        )),
                        TupleEntry::Value(DatexExpression::Integer(
                            Integer::from(3)
                        )),
                    ]))
                ],
            )
        );
    }

    #[test]
    fn test_apply_atom() {
        let src = "print 'test'";
        let expr = parse_unwrap(src);
        assert_eq!(
            expr,
            DatexExpression::ApplyChain(
                Box::new(DatexExpression::Literal {
                    name: "print".to_string(),
                    variant: None,
                }),
                vec![ApplyOperation::FunctionCall(DatexExpression::Text(
                    "test".to_string()
                ))],
            )
        );
    }

    #[test]
    fn test_property_access() {
        let src = "myObj.myProp";
        let expr = parse_unwrap(src);
        assert_eq!(
            expr,
            DatexExpression::ApplyChain(
                Box::new(DatexExpression::Literal {
                    name: "myObj".to_string(),
                    variant: None,
                }),
                vec![ApplyOperation::PropertyAccess(DatexExpression::Text(
                    "myProp".to_string()
                ))],
            )
        );
    }

    #[test]
    fn test_property_access_scoped() {
        let src = "myObj.(1)";
        let expr = parse_unwrap(src);
        assert_eq!(
            expr,
            DatexExpression::ApplyChain(
                Box::new(DatexExpression::Literal {
                    name: "myObj".to_string(),
                    variant: None,
                }),
                vec![ApplyOperation::PropertyAccess(DatexExpression::Integer(
                    Integer::from(1)
                ))],
            )
        );
    }

    #[test]
    fn test_property_access_multiple() {
        let src = "myObj.myProp.anotherProp.(1 + 2).(x;y)";
        let expr = parse_unwrap(src);
        assert_eq!(
            expr,
            DatexExpression::ApplyChain(
                Box::new(DatexExpression::Literal {
                    name: "myObj".to_string(),
                    variant: None,
                }),
                vec![
                    ApplyOperation::PropertyAccess(DatexExpression::Text(
                        "myProp".to_string()
                    )),
                    ApplyOperation::PropertyAccess(DatexExpression::Text(
                        "anotherProp".to_string()
                    )),
                    ApplyOperation::PropertyAccess(
                        DatexExpression::BinaryOperation(
                            BinaryOperator::Add,
                            Box::new(DatexExpression::Integer(Integer::from(
                                1
                            ))),
                            Box::new(DatexExpression::Integer(Integer::from(
                                2
                            ))),
                        )
                    ),
                    ApplyOperation::PropertyAccess(
                        DatexExpression::Statements(vec![
                            Statement {
                                expression: DatexExpression::Literal {
                                    name: "x".to_string(),
                                    variant: None,
                                },
                                is_terminated: true,
                            },
                            Statement {
                                expression: DatexExpression::Literal {
                                    name: "y".to_string(),
                                    variant: None,
                                },
                                is_terminated: false,
                            },
                        ])
                    ),
                ],
            )
        );
    }

    #[test]
    fn test_property_access_and_apply() {
        let src = "myObj.myProp(1, 2)";
        let expr = parse_unwrap(src);
        assert_eq!(
            expr,
            DatexExpression::ApplyChain(
                Box::new(DatexExpression::Literal {
                    name: "myObj".to_string(),
                    variant: None,
                }),
                vec![
                    ApplyOperation::PropertyAccess(DatexExpression::Text(
                        "myProp".to_string()
                    )),
                    ApplyOperation::FunctionCall(DatexExpression::Tuple(vec![
                        TupleEntry::Value(DatexExpression::Integer(
                            Integer::from(1)
                        )),
                        TupleEntry::Value(DatexExpression::Integer(
                            Integer::from(2)
                        )),
                    ])),
                ],
            )
        );
    }

    #[test]
    fn test_apply_and_property_access() {
        let src = "myFunc(1).myProp";
        let expr = parse_unwrap(src);
        assert_eq!(
            expr,
            DatexExpression::ApplyChain(
                Box::new(DatexExpression::Literal {
                    name: "myFunc".to_string(),
                    variant: None,
                }),
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
                        Box::new(DatexExpression::Literal {
                            name: "x".to_string(),
                            variant: None,
                        }),
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
                    reference_mutability: ReferenceMutability::None,
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
                reference_mutability: ReferenceMutability::None,
                type_annotation: None,
                name: "x".to_string(),
                value: Box::new(DatexExpression::BinaryOperation(
                    BinaryOperator::Add,
                    Box::new(DatexExpression::Integer(Integer::from(1))),
                    Box::new(DatexExpression::Integer(Integer::from(2))),
                )),
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
                Box::new(DatexExpression::Literal {
                    name: "myFunc".to_string(),
                    variant: None,
                }),
                vec![ApplyOperation::FunctionCall(DatexExpression::Integer(
                    Integer::from(1)
                ))]
            ),])
        );
    }

    #[test]
    fn test_fraction() {
        let src = "1/3";
        let val = try_parse_to_value_container(src);
        assert_eq!(val, ValueContainer::from(Decimal::from_string("1/3")));

        let res = parse("42.4/3");
        assert!(res.is_err());
        let res = parse("42 /3");
        assert!(res.is_err());
        let res = parse("42/ 3");
        assert!(res.is_err());
    }

    #[test]
    fn test_endpoint() {
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
                        reference_mutability: ReferenceMutability::None,
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
                            BinaryOperator::Multiply,
                            Box::new(DatexExpression::Integer(Integer::from(
                                100
                            ))),
                            Box::new(DatexExpression::Integer(Integer::from(
                                10
                            ))),
                        )),
                    ),
                    is_terminated: true,
                },
            ])
        );
    }

    #[test]
    fn test_placeholder() {
        let src = "?";
        let expr = parse_unwrap(src);
        assert_eq!(expr, DatexExpression::Placeholder);
    }

    #[test]
    fn test_integer_to_value_container() {
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
    fn test_decimal_to_value_container() {
        let src = "123.456789123456";
        let val = try_parse_to_value_container(src);
        assert_eq!(
            val,
            ValueContainer::from(Decimal::from_string("123.456789123456"))
        );
    }

    #[test]
    fn test_text_to_value_container() {
        let src = r#""Hello, world!""#;
        let val = try_parse_to_value_container(src);
        assert_eq!(val, ValueContainer::from("Hello, world!".to_string()));
    }

    #[test]
    fn test_array_to_value_container() {
        let src = "[1, 2, 3, 4.5, \"text\"]";
        let val = try_parse_to_value_container(src);
        let value_container_array: Vec<ValueContainer> = vec![
            Integer::from(1).into(),
            Integer::from(2).into(),
            Integer::from(3).into(),
            Decimal::from_string("4.5").into(),
            "text".to_string().into(),
        ];
        assert_eq!(val, ValueContainer::from(value_container_array));
    }

    #[test]
    fn test_json_to_value_container() {
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
            Decimal::from_string("0.5").into(),
        ];
        let value_container_inner_object: ValueContainer =
            ValueContainer::from(Object::from(
                vec![("key".to_string(), "value".to_string().into())]
                    .into_iter()
                    .collect::<HashMap<String, ValueContainer>>(),
            ));
        let value_container_object: ValueContainer =
            ValueContainer::from(Object::from(
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
    fn test_invalid_value_containers() {
        let src = "1 + 2";
        let expr = parse_unwrap(src);
        assert!(
            ValueContainer::try_from(expr).is_err(),
            "Expected error when converting expression to ValueContainer"
        );

        let src = "xy";
        let expr = parse_unwrap(src);
        assert!(
            ValueContainer::try_from(expr).is_err(),
            "Expected error when converting expression to ValueContainer"
        );

        let src = "x()";
        let expr = parse_unwrap(src);
        assert!(
            ValueContainer::try_from(expr).is_err(),
            "Expected error when converting expression to ValueContainer"
        );
    }

    #[test]
    fn test_invalid_add() {
        let src = "1+2";
        let res = parse(src);
        println!("res: {res:?}");
        assert!(
            res.unwrap_err().len() == 1,
            "Expected error when parsing expression"
        );
    }

    #[test]
    fn test_decimal_nan() {
        let src = "NaN";
        let num = parse_unwrap(src);
        assert_matches!(num, DatexExpression::Decimal(Decimal::NaN));

        let src = "nan";
        let num = parse_unwrap(src);
        assert_matches!(num, DatexExpression::Decimal(Decimal::NaN));
    }

    #[test]
    fn test_decimal_infinity() {
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
    fn test_comment() {
        let src = "// This is a comment\n1 + 2";
        let expr = parse_unwrap(src);
        assert_eq!(
            expr,
            DatexExpression::BinaryOperation(
                BinaryOperator::Add,
                Box::new(DatexExpression::Integer(Integer::from(1))),
                Box::new(DatexExpression::Integer(Integer::from(2))),
            )
        );

        let src = "1 + //test\n2";
        let expr = parse_unwrap(src);
        assert_eq!(
            expr,
            DatexExpression::BinaryOperation(
                BinaryOperator::Add,
                Box::new(DatexExpression::Integer(Integer::from(1))),
                Box::new(DatexExpression::Integer(Integer::from(2))),
            )
        );
    }

    #[test]
    fn test_multiline_comment() {
        let src = "/* This is a\nmultiline comment */\n1 + 2";
        let expr = parse_unwrap(src);
        assert_eq!(
            expr,
            DatexExpression::BinaryOperation(
                BinaryOperator::Add,
                Box::new(DatexExpression::Integer(Integer::from(1))),
                Box::new(DatexExpression::Integer(Integer::from(2))),
            )
        );

        let src = "1 + /*test*/ 2";
        let expr = parse_unwrap(src);
        assert_eq!(
            expr,
            DatexExpression::BinaryOperation(
                BinaryOperator::Add,
                Box::new(DatexExpression::Integer(Integer::from(1))),
                Box::new(DatexExpression::Integer(Integer::from(2))),
            )
        );
    }

    #[test]
    fn test_shebang() {
        let src = "#!/usr/bin/env datex\n1 + 2";
        let expr = parse_unwrap(src);
        assert_eq!(
            expr,
            DatexExpression::BinaryOperation(
                BinaryOperator::Add,
                Box::new(DatexExpression::Integer(Integer::from(1))),
                Box::new(DatexExpression::Integer(Integer::from(2))),
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
    fn test_remote_execution() {
        let src = "a :: b";
        let expr = parse_unwrap(src);
        assert_eq!(
            expr,
            DatexExpression::RemoteExecution(
                Box::new(DatexExpression::Literal {
                    name: "a".to_string(),
                    variant: None,
                }),
                Box::new(DatexExpression::Literal {
                    name: "b".to_string(),
                    variant: None,
                })
            )
        );
    }
    #[test]
    fn test_remote_execution_no_space() {
        let src = "a::b";
        let expr = parse_unwrap(src);
        assert_eq!(
            expr,
            DatexExpression::RemoteExecution(
                Box::new(DatexExpression::Literal {
                    name: "a".to_string(),
                    variant: None,
                }),
                Box::new(DatexExpression::Literal {
                    name: "b".to_string(),
                    variant: None,
                })
            )
        );
    }

    #[test]
    fn test_remote_execution_complex() {
        let src = "a :: b + c * 2";
        let expr = parse_unwrap(src);
        assert_eq!(
            expr,
            DatexExpression::RemoteExecution(
                Box::new(DatexExpression::Literal {
                    name: "a".to_string(),
                    variant: None,
                }),
                Box::new(DatexExpression::BinaryOperation(
                    BinaryOperator::Add,
                    Box::new(DatexExpression::Literal {
                        name: "b".to_string(),
                        variant: None,
                    }),
                    Box::new(DatexExpression::BinaryOperation(
                        BinaryOperator::Multiply,
                        Box::new(DatexExpression::Literal {
                            name: "c".to_string(),
                            variant: None,
                        }),
                        Box::new(DatexExpression::Integer(Integer::from(2))),
                    )),
                )),
            )
        );
    }

    #[test]
    fn test_remote_execution_statements() {
        let src = "a :: b; 1";
        let expr = parse_unwrap(src);
        assert_eq!(
            expr,
            DatexExpression::Statements(vec![
                Statement {
                    expression: DatexExpression::RemoteExecution(
                        Box::new(DatexExpression::Literal {
                            name: "a".to_string(),
                            variant: None,
                        }),
                        Box::new(DatexExpression::Literal {
                            name: "b".to_string(),
                            variant: None,
                        })
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
    fn test_remote_execution_inline_statements() {
        let src = "a :: (1; 2 + 3)";
        let expr = parse_unwrap(src);
        assert_eq!(
            expr,
            DatexExpression::RemoteExecution(
                Box::new(DatexExpression::Literal {
                    name: "a".to_string(),
                    variant: None,
                }),
                Box::new(DatexExpression::Statements(vec![
                    Statement {
                        expression: DatexExpression::Integer(Integer::from(1)),
                        is_terminated: true,
                    },
                    Statement {
                        expression: DatexExpression::BinaryOperation(
                            BinaryOperator::Add,
                            Box::new(DatexExpression::Integer(Integer::from(
                                2
                            ))),
                            Box::new(DatexExpression::Integer(Integer::from(
                                3
                            ))),
                        ),
                        is_terminated: false,
                    },
                ])),
            )
        );
    }

    #[test]
    fn test_named_slot() {
        let src = "#endpoint";
        let expr = parse_unwrap(src);
        assert_eq!(
            expr,
            DatexExpression::Slot(Slot::Named("endpoint".to_string()))
        );
    }

    #[test]
    fn test_addressed_slot() {
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
                reference_mutability: ReferenceMutability::Mutable,
                name: "x".to_string(),
                type_annotation: None,
                value: Box::new(DatexExpression::Array(vec![
                    DatexExpression::Integer(Integer::from(1)),
                    DatexExpression::Integer(Integer::from(2)),
                    DatexExpression::Integer(Integer::from(3)),
                ])),
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
                reference_mutability: ReferenceMutability::Immutable,
                name: "x".to_string(),
                type_annotation: None,
                value: Box::new(DatexExpression::Array(vec![
                    DatexExpression::Integer(Integer::from(1)),
                    DatexExpression::Integer(Integer::from(2)),
                    DatexExpression::Integer(Integer::from(3)),
                ])),
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
                reference_mutability: ReferenceMutability::None,
                name: "x".to_string(),
                type_annotation: None,
                value: Box::new(DatexExpression::Integer(Integer::from(1))),
            }
        );
    }
}
