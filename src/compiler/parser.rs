use crate::compiler::lexer::Token;
use crate::compiler::parser::extra::Err;
use crate::datex_values::core_values::array::Array;
use crate::datex_values::core_values::decimal::decimal::Decimal;
use crate::datex_values::core_values::integer::integer::Integer;
use crate::datex_values::core_values::object::Object;
use crate::datex_values::value::Value;
use crate::datex_values::value_container::ValueContainer;
use crate::global::binary_codes::InstructionCode;
use chumsky::{
    input::{Stream, ValueInput},
    prelude::*,
};
use logos::Logos;
use std::{collections::HashMap, ops::Range};

#[derive(Clone, Debug, PartialEq)]
pub enum TupleEntry {
    KeyValue(DatexExpression, DatexExpression),
    Value(DatexExpression),
}

#[derive(Clone, Debug, PartialEq)]
pub enum BinaryOperator {
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,
    Power,
    And,
    Or,
    CompositeAnd,
    CompositeOr,
    Equal,
    NotEqual,
    StrictEqual,
    StrictNotEqual,
    Identical,
    NotIdentical,
    LessThan,
    GreaterThan,
    LessThanOrEqual,
    GreaterThanOrEqual,
}

impl From<&BinaryOperator> for InstructionCode {
    fn from(op: &BinaryOperator) -> Self {
        match op {
            BinaryOperator::Add => InstructionCode::ADD,
            BinaryOperator::Subtract => InstructionCode::SUBTRACT,
            BinaryOperator::Multiply => InstructionCode::MULTIPLY,
            BinaryOperator::Divide => InstructionCode::DIVIDE,
            BinaryOperator::Modulo => InstructionCode::MODULO,
            BinaryOperator::Power => InstructionCode::POWER,
            BinaryOperator::And => InstructionCode::AND,
            BinaryOperator::Or => InstructionCode::OR,
            BinaryOperator::Equal => InstructionCode::EQUAL_VALUE,
            BinaryOperator::StrictEqual => InstructionCode::EQUAL,
            _ => todo!(),
        }
    }
}

impl From<BinaryOperator> for InstructionCode {
    fn from(op: BinaryOperator) -> Self {
        InstructionCode::from(&op)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum UnaryOperator {
    Negate,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Statement {
    pub expression: DatexExpression,
    pub is_terminated: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Apply {
    /// Apply a function to an argument
    FunctionCall(DatexExpression),
    /// Apply a property access to an argument
    PropertyAccess(DatexExpression),
}

#[derive(Clone, Debug, PartialEq)]
pub enum VariableType {
    Value,
    Reference,
}

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
    /// Integer, e.g 123456789123456789
    Integer(Integer),
    /// Array, e.g  `[1, 2, 3, "text"]`
    Array(Vec<DatexExpression>),
    /// Object, e.g {"key": "value", key2: 2}
    Object(Vec<(DatexExpression, DatexExpression)>),
    /// Tuple, e.g (1: 2, 3: 4, "xy") or without brackets: 1,2,a:3
    Tuple(Vec<TupleEntry>),
    /// One or more statements, e.g (1; 2; 3)
    Statements(Vec<Statement>),
    /// Identifier, e.g. a variable name
    Variable(String),
    /// Variable declaration, e.g. ref x = 1 or val y = 2
    VariableDeclaration(VariableType, String, Box<DatexExpression>),
    /// Variable assignment, e.g. x = 1
    VariableAssignment(String, Box<DatexExpression>),

    BinaryOperation(BinaryOperator, Box<DatexExpression>, Box<DatexExpression>),
    UnaryOperation(UnaryOperator, Box<DatexExpression>),

    // apply (e.g. x (1)) or property access
    ApplyChain(Box<DatexExpression>, Vec<Apply>),
    // ?
    Placeholder,
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

pub type DatexScriptParser<'a> =
    Boxed<'a, 'a, TokenInput<'a>, DatexExpression, Err<Rich<'a, Token>>>;

fn decode_json_unicode_escapes(input: &str) -> String {
    let mut output = String::new();
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\\' && chars.peek() == Some(&'u') {
            chars.next(); // skip 'u'

            let mut code_unit = String::new();
            for _ in 0..4 {
                if let Some(c) = chars.next() {
                    code_unit.push(c);
                } else {
                    output.push_str("\\u");
                    output.push_str(&code_unit);
                    break;
                }
            }

            if let Ok(first_unit) = u16::from_str_radix(&code_unit, 16) {
                if (0xD800..=0xDBFF).contains(&first_unit) {
                    // High surrogate — look for low surrogate
                    if chars.next() == Some('\\') && chars.next() == Some('u') {
                        let mut low_code = String::new();
                        for _ in 0..4 {
                            if let Some(c) = chars.next() {
                                low_code.push(c);
                            } else {
                                output.push_str(&format!(
                                    "\\u{first_unit:04X}\\u{low_code}"
                                ));
                                break;
                            }
                        }

                        if let Ok(second_unit) =
                            u16::from_str_radix(&low_code, 16)
                            && (0xDC00..=0xDFFF).contains(&second_unit)
                        {
                            let combined = 0x10000
                                + (((first_unit - 0xD800) as u32) << 10)
                                + ((second_unit - 0xDC00) as u32);
                            if let Some(c) = char::from_u32(combined) {
                                output.push(c);
                                continue;
                            }
                        }

                        // Invalid surrogate fallback
                        output.push_str(&format!(
                            "\\u{first_unit:04X}\\u{low_code}"
                        ));
                    } else {
                        // Unpaired high surrogate
                        output.push_str(&format!("\\u{first_unit:04X}"));
                    }
                } else {
                    // Normal scalar value
                    if let Some(c) = char::from_u32(first_unit as u32) {
                        output.push(c);
                    } else {
                        output.push_str(&format!("\\u{first_unit:04X}"));
                    }
                }
            } else {
                output.push_str(&format!("\\u{code_unit}"));
            }
        } else {
            output.push(ch);
        }
    }

    output
}

/// Takes a literal text string input, e.g. ""Hello, world!"" or "'Hello, world!' or ""x\"""
/// and returns the unescaped text, e.g. "Hello, world!" or 'Hello, world!' or "x\""
fn unescape_text(text: &str) -> String {
    // remove first and last quote (double or single)
    let escaped = text[1..text.len() - 1]
        // Replace escape sequences with actual characters
        .replace(r#"\""#, "\"") // Replace \" with "
        .replace(r#"\'"#, "'") // Replace \' with '
        .replace(r#"\n"#, "\n") // Replace \n with newline
        .replace(r#"\r"#, "\r") // Replace \r with carriage return
        .replace(r#"\t"#, "\t") // Replace \t with tab
        .replace(r#"\b"#, "\x08") // Replace \b with backspace
        .replace(r#"\f"#, "\x0C") // Replace \f with form feed
        .replace(r#"\\"#, "\\") // Replace \\ with \
        // TODO remove all other backslashes before any other character
        .to_string();
    // Decode unicode escapes, e.g. \u1234 or \uD800\uDC00
    decode_json_unicode_escapes(&escaped)
}

fn binary_op(
    op: BinaryOperator,
) -> impl Fn(Box<DatexExpression>, Box<DatexExpression>) -> DatexExpression + Clone
{
    move |lhs, rhs| DatexExpression::BinaryOperation(op.clone(), lhs, rhs)
}

pub struct DatexParseResult {
    pub expression: DatexExpression,
    pub is_static_value: bool,
}

pub fn create_parser<'a, I>() -> impl Parser<'a, I, DatexExpression, Err<Cheap>>
where
    I: ValueInput<'a, Token = Token, Span = SimpleSpan>,
{
    // an expression
    let mut expression = Recursive::declare();
    let mut expression_without_tuple = Recursive::declare();

    let whitespace = just(Token::Whitespace).repeated().ignored();

    // a sequence of expressions, separated by semicolons, optionally terminated with a semicolon
    let statements = expression
        .clone()
        .then_ignore(
            just(Token::Semicolon)
                .padded_by(whitespace.clone())
                .repeated()
                .at_least(1),
        )
        .repeated()
        .collect::<Vec<_>>()
        .then(
            expression
                .clone()
                .then(
                    just(Token::Semicolon)
                        .padded_by(whitespace.clone())
                        .or_not(),
                )
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

    // primitive values (e.g. 1, "text", true, null)
    let integer = select! {
        Token::IntegerLiteral(s) => DatexExpression::Integer(Integer::from_string(&s).unwrap()),
        Token::BinaryIntegerLiteral(s) => DatexExpression::Integer(Integer::from_string_radix(&s[2..], 2).unwrap()),
        Token::HexadecimalIntegerLiteral(s) => DatexExpression::Integer(Integer::from_string_radix(&s[2..], 16).unwrap()),
        Token::OctalIntegerLiteral(s) => DatexExpression::Integer(Integer::from_string_radix(&s[2..], 8).unwrap()),
    };
    let decimal = select! {
        Token::DecimalLiteral(s) => DatexExpression::Decimal(Decimal::from_string(&s)),
        Token::NanLiteral => DatexExpression::Decimal(Decimal::NaN),
        Token::InfinityLiteral(s) => DatexExpression::Decimal(
            if s.starts_with('-') {
                Decimal::NegInfinity
            } else {
                Decimal::Infinity
            }
        ),
        Token::FractionLiteral(s) => DatexExpression::Decimal(Decimal::from_string(&s)),
    };
    let text = select! {
        Token::StringLiteral(s) => DatexExpression::Text(unescape_text(&s))
    };
    let literal = select! {
        Token::TrueKW => DatexExpression::Boolean(true),
        Token::FalseKW => DatexExpression::Boolean(false),
        Token::NullKW => DatexExpression::Null,
        Token::Identifier(s) => DatexExpression::Variable(s),
        Token::PlaceholderKW => DatexExpression::Placeholder,
    };
    // expression wrapped in parentheses
    let wrapped_expression = statements
        .clone()
        .delimited_by(just(Token::LeftParen), just(Token::RightParen));

    // a valid object/tuple key
    // (1: value), "key", 1, (("x"+"y"): 123)
    let key = choice((
        text,
        decimal,
        integer,
        // any valid identifiers (equivalent to variable names), mapped to a text
        select! {
            Token::Identifier(s) => DatexExpression::Text(s)
        },
        // dynamic key
        wrapped_expression.clone(),
    ));

    // array
    // 1,2,3
    // [1,2,3,4,13434,(1),4,5,7,8]
    let array = expression_without_tuple
        .clone()
        .separated_by(just(Token::Comma).padded_by(whitespace.clone()))
        .at_least(0)
        .allow_trailing()
        .collect()
        .padded_by(whitespace.clone())
        .delimited_by(just(Token::LeftBracket), just(Token::RightBracket))
        .map(DatexExpression::Array);

    // object
    let object = key
        .clone()
        .then_ignore(just(Token::Colon).padded_by(whitespace.clone()))
        .then(expression_without_tuple.clone())
        .separated_by(just(Token::Comma).padded_by(whitespace.clone()))
        .at_least(0)
        .allow_trailing()
        .collect()
        .padded_by(whitespace.clone())
        .delimited_by(just(Token::LeftCurly), just(Token::RightCurly))
        .map(DatexExpression::Object);

    // tuple
    // Key-value pair
    let tuple_key_value_pair = key
        .clone()
        .then_ignore(just(Token::Colon).padded_by(whitespace.clone()))
        .then(expression_without_tuple.clone())
        .map(|(key, value)| TupleEntry::KeyValue(key, value));

    // tuple (either key:value entries or just values)
    let tuple_entry = choice((
        // Key-value pair
        tuple_key_value_pair.clone(),
        // Just a value with no key
        expression_without_tuple.clone().map(TupleEntry::Value),
    ))
    .boxed();

    let tuple = tuple_entry
        .clone()
        .separated_by(just(Token::Comma).padded_by(whitespace.clone()))
        .at_least(2)
        .collect::<Vec<_>>()
        .map(DatexExpression::Tuple);

    // e.g. x,
    let single_value_tuple = tuple_entry
        .clone()
        .then_ignore(just(Token::Comma))
        .map(|value| vec![value])
        .map(DatexExpression::Tuple);

    // e.g. (a:1)
    let single_keyed_tuple_entry = tuple_key_value_pair
        .clone()
        .map(|value| vec![value])
        .map(DatexExpression::Tuple);

    let tuple = choice((tuple, single_value_tuple, single_keyed_tuple_entry));

    // atomic expression (e.g. 1, "text", (1 + 2), (1;2))
    let atom = choice((
        array.clone(),
        object.clone(),
        literal,
        decimal,
        integer,
        text,
        wrapped_expression.clone(),
    ))
    .boxed();

    // operations on atoms
    let op = |c| {
        just(Token::Whitespace)
            .repeated()
            .at_least(1)
            .ignore_then(just(c))
            .then_ignore(just(Token::Whitespace).repeated().at_least(1))
    };

    // apply chain: two expressions following each other directly, optionally separated with "." (property access)
    let apply_or_property_access = atom
        .clone()
        .then(
            choice((
                // apply #1: a wrapped expression, array, or object - no whitespace required before
                // x () x [] x {}
                choice((
                    wrapped_expression.clone(),
                    array.clone(),
                    object.clone(),
                ))
                .clone()
                .padded_by(whitespace.clone())
                .map(Apply::FunctionCall),
                // apply #2: an atomic value (e.g. "text") - whitespace or newline required before
                // print "sdf"
                just(Token::Whitespace)
                    .repeated()
                    .at_least(1)
                    .ignore_then(atom.clone().padded_by(whitespace.clone()))
                    .map(Apply::FunctionCall),
                // property access
                just(Token::Dot)
                    .padded_by(whitespace.clone())
                    .ignore_then(key.clone())
                    .map(Apply::PropertyAccess),
            ))
            .repeated()
            .collect::<Vec<_>>(),
        )
        .map(|(val, args)| {
            // if only single value, return it directly
            if args.is_empty() {
                val
            } else {
                DatexExpression::ApplyChain(Box::new(val), args)
            }
        });

    let product = apply_or_property_access.clone().foldl(
        choice((
            op(Token::Star).to(binary_op(BinaryOperator::Multiply)),
            op(Token::Slash).to(binary_op(BinaryOperator::Divide)),
        ))
        .then(apply_or_property_access)
        .repeated(),
        |lhs, (op, rhs)| op(Box::new(lhs), Box::new(rhs)),
    );

    let sum = product.clone().foldl(
        choice((
            op(Token::Plus).to(binary_op(BinaryOperator::Add)),
            op(Token::Minus).to(binary_op(BinaryOperator::Subtract)),
        ))
        .then(product)
        .repeated(),
        |lhs, (op, rhs)| op(Box::new(lhs), Box::new(rhs)),
    );

    // variable declarations or assignments
    let variable_assignment = just(Token::ValKW)
        .or(just(Token::RefKW))
        .or_not()
        .padded_by(whitespace.clone())
        .then(select! {
            Token::Identifier(s) => s
        })
        .then_ignore(just(Token::Assign).padded_by(whitespace.clone()))
        .then(sum.clone())
        .map(|((var_type, var_name), expr)| {
            if let Some(var_type) = var_type {
                DatexExpression::VariableDeclaration(
                    if var_type == Token::ValKW {
                        VariableType::Value
                    } else {
                        VariableType::Reference
                    },
                    var_name.to_string(),
                    Box::new(expr),
                )
            } else {
                DatexExpression::VariableAssignment(
                    var_name.to_string(),
                    Box::new(expr),
                )
            }
        });

    expression_without_tuple.define(choice((variable_assignment, sum.clone())));

    expression.define(
        choice((tuple.clone(), expression_without_tuple.clone()))
            .padded_by(whitespace.clone()),
    );

    choice((
        // empty script (0-n semicolons)
        just(Token::Semicolon)
            .repeated()
            .at_least(1)
            .padded_by(whitespace.clone())
            .map(|_| DatexExpression::Statements(vec![])),
        // statements
        statements,
    ))
}

type TokenInput<'a> = &'a [Token];

#[derive(Debug)]
pub enum ParserError {
    UnexpectedToken(Range<usize>), //(Rich<'a, Token>),
    InvalidToken(Range<usize>),
}

pub fn parse(src: &str) -> Result<DatexExpression, Vec<ParserError>> {
    let token_iter = Token::lexer(src).spanned().map(|(tok, span)| match tok {
        Ok(tok) => (tok, span.into()),
        Err(_) => (Token::Error, span.into()),
    });
    let token_stream = Stream::from_iter(token_iter)
        .map((0..src.len()).into(), |(t, s): (_, _)| (t, s));

    let result =
        create_parser()
            .parse(token_stream)
            .into_result()
            .map_err(|err| {
                err.into_iter()
                    .map(|e| {
                        ParserError::UnexpectedToken(e.span().into_range())
                    })
                    .collect()
            });
    result
}

#[cfg(test)]
mod tests {

    use super::*;
    
    use std::assert_matches::assert_matches;

    fn print_report(errs: Vec<ParserError>, src: &str) {
        // FIXME
        eprintln!("{errs:?}");
        // errs.into_iter().for_each(|e| {
        //     Report::build(ReportKind::Error, ((), e.span().into_range()))
        //         .with_config(
        //             ariadne::Config::new()
        //                 .with_index_type(ariadne::IndexType::Byte),
        //         )
        //         .with_message(e.to_string())
        //         .with_label(
        //             Label::new(((), e.span().into_range()))
        //                 .with_color(Color::Red),
        //         )
        //         .finish()
        //         .eprint(Source::from(&src))
        //         .unwrap()
        // });
    }

    fn parse_unwrap(src: &str) -> DatexExpression {
        let res = parse(src);
        if res.is_err() {
            print_report(res.unwrap_err(), src);
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

    // #[test]
    // fn test_equal_operator() {
    //     let src = "1 == 1";
    //     let val = parse_unwrap(src);
    //     // assert_eq!(val, DatexExpression::Null);
    // }

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
                    Box::new(DatexExpression::Variable("x".to_string())),
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
        assert_eq!(expr, DatexExpression::Variable("myVar".to_string()));
    }

    #[test]
    fn test_variable_expression_with_operations() {
        let src = "myVar + 1";
        let expr = parse_unwrap(src);
        assert_eq!(
            expr,
            DatexExpression::BinaryOperation(
                BinaryOperator::Add,
                Box::new(DatexExpression::Variable("myVar".to_string())),
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
                Box::new(DatexExpression::Variable("myFunc".to_string())),
                vec![Apply::FunctionCall(DatexExpression::Tuple(vec![
                    TupleEntry::Value(DatexExpression::Integer(Integer::from(
                        1
                    ))),
                    TupleEntry::Value(DatexExpression::Integer(Integer::from(
                        2
                    ))),
                    TupleEntry::Value(DatexExpression::Integer(Integer::from(
                        3
                    ))),
                ]),)],
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
                Box::new(DatexExpression::Variable("myFunc".to_string())),
                vec![Apply::FunctionCall(DatexExpression::Statements(vec![]))],
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
                Box::new(DatexExpression::Variable("myFunc".to_string())),
                vec![
                    Apply::FunctionCall(DatexExpression::Integer(
                        Integer::from(1)
                    ),),
                    Apply::FunctionCall(DatexExpression::Tuple(vec![
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
                Box::new(DatexExpression::Variable("print".to_string())),
                vec![Apply::FunctionCall(DatexExpression::Text(
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
                Box::new(DatexExpression::Variable("myObj".to_string())),
                vec![Apply::PropertyAccess(DatexExpression::Text(
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
                Box::new(DatexExpression::Variable("myObj".to_string())),
                vec![Apply::PropertyAccess(DatexExpression::Integer(
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
                Box::new(DatexExpression::Variable("myObj".to_string())),
                vec![
                    Apply::PropertyAccess(DatexExpression::Text(
                        "myProp".to_string()
                    )),
                    Apply::PropertyAccess(DatexExpression::Text(
                        "anotherProp".to_string()
                    )),
                    Apply::PropertyAccess(DatexExpression::BinaryOperation(
                        BinaryOperator::Add,
                        Box::new(DatexExpression::Integer(Integer::from(1))),
                        Box::new(DatexExpression::Integer(Integer::from(2))),
                    )),
                    Apply::PropertyAccess(DatexExpression::Statements(vec![
                        Statement {
                            expression: DatexExpression::Variable(
                                "x".to_string()
                            ),
                            is_terminated: true,
                        },
                        Statement {
                            expression: DatexExpression::Variable(
                                "y".to_string()
                            ),
                            is_terminated: false,
                        },
                    ])),
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
                Box::new(DatexExpression::Variable("myObj".to_string())),
                vec![
                    Apply::PropertyAccess(DatexExpression::Text(
                        "myProp".to_string()
                    )),
                    Apply::FunctionCall(DatexExpression::Tuple(vec![
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
                Box::new(DatexExpression::Variable("myFunc".to_string())),
                vec![
                    Apply::FunctionCall(DatexExpression::Integer(
                        Integer::from(1)
                    )),
                    Apply::PropertyAccess(DatexExpression::Text(
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
                        Box::new(DatexExpression::Variable("x".to_string())),
                        vec![Apply::FunctionCall(DatexExpression::Integer(
                            Integer::from(1)
                        ))],
                    )),
                    vec![Apply::PropertyAccess(DatexExpression::Text(
                        "y".to_string()
                    ))],
                )),
                vec![Apply::PropertyAccess(DatexExpression::Text(
                    "z".to_string()
                ))],
            )
        );
    }

    #[test]
    fn variable_declaration() {
        let src = "val x = 42";
        let expr = parse_unwrap(src);
        assert_eq!(
            expr,
            DatexExpression::VariableDeclaration(
                VariableType::Value,
                "x".to_string(),
                Box::new(DatexExpression::Integer(Integer::from(42))),
            )
        );
    }

    #[test]
    fn variable_declaration_statement() {
        let src = "val x = 42;";
        let expr = parse_unwrap(src);
        assert_eq!(
            expr,
            DatexExpression::Statements(vec![Statement {
                expression: DatexExpression::VariableDeclaration(
                    VariableType::Value,
                    "x".to_string(),
                    Box::new(DatexExpression::Integer(Integer::from(42))),
                ),
                is_terminated: true,
            },])
        );
    }

    #[test]
    fn variable_declaration_with_expression() {
        let src = "ref x = 1 + 2";
        let expr = parse_unwrap(src);
        assert_eq!(
            expr,
            DatexExpression::VariableDeclaration(
                VariableType::Reference,
                "x".to_string(),
                Box::new(DatexExpression::BinaryOperation(
                    BinaryOperator::Add,
                    Box::new(DatexExpression::Integer(Integer::from(1))),
                    Box::new(DatexExpression::Integer(Integer::from(2))),
                )),
            )
        );
    }

    #[test]
    fn variable_assignment() {
        let src = "x = 42";
        let expr = parse_unwrap(src);
        assert_eq!(
            expr,
            DatexExpression::VariableAssignment(
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
            DatexExpression::VariableAssignment(
                "x".to_string(),
                Box::new(DatexExpression::VariableAssignment(
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
            DatexExpression::Array(vec![DatexExpression::VariableAssignment(
                "x".to_string(),
                Box::new(DatexExpression::Integer(Integer::from(1))),
            ),])
        );
    }

    #[test]
    fn apply_in_array() {
        let src = "[myFunc(1)]";
        let expr = parse_unwrap(src);
        assert_eq!(
            expr,
            DatexExpression::Array(vec![DatexExpression::ApplyChain(
                Box::new(DatexExpression::Variable("myFunc".to_string())),
                vec![Apply::FunctionCall(DatexExpression::Integer(
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

    // TODO:
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
        let src = "val x = 42; x = 100 * 10;";
        let expr = parse_unwrap(src);
        assert_eq!(
            expr,
            DatexExpression::Statements(vec![
                Statement {
                    expression: DatexExpression::VariableDeclaration(
                        VariableType::Value,
                        "x".to_string(),
                        Box::new(DatexExpression::Integer(Integer::from(42))),
                    ),
                    is_terminated: true,
                },
                Statement {
                    expression: DatexExpression::VariableAssignment(
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
}
