use std::collections::HashMap;
use crate::compiler::parser::extra::Err;
use chumsky::prelude::*;
use crate::datex_values::core_values::array::Array;
use crate::datex_values::core_values::decimal::decimal::Decimal;
use crate::datex_values::core_values::integer::integer::Integer;
use crate::datex_values::core_values::object::Object;
use crate::datex_values::value::Value;
use crate::datex_values::value_container::ValueContainer;
use crate::global::binary_codes::InstructionCode;

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
            _ => todo!()
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
                let entries = arr.into_iter()
                    .map(ValueContainer::try_from)
                    .collect::<Result<Vec<ValueContainer>, ()>>()?;
                ValueContainer::from(Array::from(entries))
            },
            DatexExpression::Object(obj) => {
                let entries = obj.into_iter()
                    .map(|(k, v)| {
                        let key = match k {
                            DatexExpression::Text(s) => s,
                            _ => Err(())?
                        };
                        let value = ValueContainer::try_from(v)?;
                        Ok((key, value))
                    })
                    .collect::<Result<HashMap<String, ValueContainer>, ()>>()?;
                ValueContainer::from(Object::from(entries))
            },
            _ => Err(())?
        })
    }
}


fn unicode_escape<'a>() -> impl Parser<'a, &'a str, char, extra::Err<Rich<'a, char>>> {
    just('u').ignore_then(text::digits(16).exactly(4).to_slice().validate(
        |digits, e, emitter| {

            let high = u16::from_str_radix(digits, 16).unwrap();
            // Check if it's a high surrogate
            if (0xD800..=0xDBFF).contains(&high) {
                // Expect a second \uXXXX
                emitter.emit(Rich::custom(e.span(), "unexpected isolated high surrogate"));
                '\u{FFFD}' // unicode replacement character
            } else if (0xDC00..=0xDFFF).contains(&high) {
                // Isolated low surrogate
                emitter.emit(Rich::custom(e.span(), "unexpected low surrogate"));
                '\u{FFFD}' // unicode replacement character
            } else {
                // Valid single unicode character
                char::from_u32(high as u32).unwrap_or_else(|| {
                    emitter.emit(Rich::custom(e.span(), "invalid unicode character"));
                    '\u{FFFD}' // unicode replacement character
                })
            }
        })
    )
}

fn unicode_surrogate_pair<'a>() -> impl Parser<'a, &'a str, char, extra::Err<Rich<'a, char>>> {
    just('u')
        .ignore_then(text::digits(16).exactly(4).to_slice())
        .then_ignore(just('\\').then(just('u')))
        .then(text::digits(16).exactly(4).to_slice())
        .validate(|(high, low), e, emitter| {
            let h = u16::from_str_radix(high, 16).unwrap();
            let l = u16::from_str_radix(low, 16).unwrap();

            if (0xD800..=0xDBFF).contains(&h) && (0xDC00..=0xDFFF).contains(&l) {
                let code_point = 0x10000 + (((h - 0xD800) as u32) << 10) + ((l - 0xDC00) as u32);
                char::from_u32(code_point).unwrap_or_else(|| {
                    emitter.emit(Rich::custom(e.span(), "invalid unicode character"));
                    '\u{FFFD}' // unicode replacement character
                })
            } else {
                emitter.emit(Rich::custom(e.span(), "invalid surrogate pair"));
                '\u{FFFD}' // unicode replacement character
            }
        })
}

pub type DatexScriptParser<'a> = Boxed<'a, 'a, &'a str, DatexExpression, Err<Rich<'a, char>>>;

fn text<'a>() -> DatexScriptParser<'a> {
    let escape = just('\\')
        .ignore_then(choice((
            just('\\'),
            just('/'),
            just('"'),
            just('b').to('\x08'),
            just('f').to('\x0C'),
            just('n').to('\n'),
            just('r').to('\r'),
            just('t').to('\t'),
            unicode_surrogate_pair(),
            unicode_escape(),
        ))).boxed();

    let text_double_quotes = none_of("\\\"")
        .or(escape.clone())
        .repeated()
        .collect::<String>()
        .delimited_by(just('"'), just('"'));

    let text_single_quotes = none_of("\\'")
        .or(escape)
        .repeated()
        .collect::<String>()
        .delimited_by(just('\''), just('\''));


    let text = choice((
        text_double_quotes.map(DatexExpression::Text),
        text_single_quotes.map(DatexExpression::Text),
    )).boxed();

    text
}
fn integer<'a>() -> DatexScriptParser<'a> {
    let dec_digits = text::digits(10)
        .then(just('_').ignore_then(text::digits(10)).repeated()).to_slice();
    let hex_digits = text::digits(16)
        .then(just('_').ignore_then(text::digits(16)).repeated()).to_slice();
    let octal_digits = text::digits(8)
        .then(just('_').ignore_then(text::digits(8)).repeated()).to_slice();
    let binary_digits = text::digits(2)
        .then(just('_').ignore_then(text::digits(2)).repeated()).to_slice();

    let integer = choice((
        // Hexadecimal integer
        just("0x").or(just("0X"))
            .ignore_then(hex_digits)
            .map(|s: &str| Integer::from_string_radix(s, 16).unwrap())
            .map(DatexExpression::Integer)
            .boxed(),
        // Octal integer
        just("0o").or(just("0O"))
            .ignore_then(octal_digits)
            .map(|s: &str| Integer::from_string_radix(s, 8).unwrap())
            .map(DatexExpression::Integer)
            .boxed(),
        // Binary integer
        just("0b").or(just("0B"))
            .ignore_then(binary_digits)
            .map(|s: &str| Integer::from_string_radix(s, 2).unwrap())
            .map(DatexExpression::Integer)
            .boxed(),
        // Decimal integer
        just('-').or_not()
            .then(dec_digits)
            .to_slice()
            .map(|s: &str| Integer::from_string(s).unwrap())
            .map(DatexExpression::Integer)
            .boxed(),
    )).boxed();
    integer
}

fn decimal<'a>() -> DatexScriptParser<'a> {
    let digits = text::digits(10).to_slice();
    let frac = just('.').then(digits);
    let exp = just('e')
        .or(just('E'))
        .then(one_of("+-").or_not())
        .then(digits);

    let decimal = just('-')
        .or_not()
        .then(choice((
            // decimal with optional exponent
            text::int(10)
                .then(frac)
                .then(exp.or_not())
                .to_slice(),
            // no decimal point, but with exponent
            digits
                .then(exp)
                .to_slice()
        )))
        .to_slice()
        .map(|s: &str| Decimal::from_string(s))
        .map(DatexExpression::Decimal)
        .boxed();

    decimal
}

fn boolean<'a>() -> DatexScriptParser<'a> {
    let true_value = just("true").to(DatexExpression::Boolean(true));
    let false_value = just("false").to(DatexExpression::Boolean(false));

    let boolean = choice((true_value, false_value)).boxed();

    boolean
}

fn null<'a>() -> DatexScriptParser<'a> {
    let null_value = just("null").to(DatexExpression::Null);
    null_value.boxed()
}

fn variable<'a>() -> DatexScriptParser<'a> {
    // valid identifiers start with _ or an ascii letter, followed by any combination of letters, digits, or underscores
    let identifier = text::ident()
        .map(|s: &str| DatexExpression::Variable(s.to_string()))
        .boxed();
    identifier
}


fn binary_op(op: BinaryOperator) -> impl Fn(Box<DatexExpression>, Box<DatexExpression>) -> DatexExpression + Clone {
    move |lhs, rhs| DatexExpression::BinaryOperation(op.clone(), lhs, rhs)
}


pub struct DatexParseResult {
    pub expression: DatexExpression,
    pub is_static_value: bool
}

pub fn create_parser<'a>() -> DatexScriptParser<'a> {
    // an expression
    let mut expression = Recursive::declare();
    let mut expression_without_tuple = Recursive::declare();
    // a sequence of expressions, separated by semicolons, optionally terminated with a semicolon
    let statements = expression.clone()
        .then_ignore(just(';').padded().repeated().at_least(1))
        .repeated()
        .collect::<Vec<_>>()
        .then(
            expression.clone().then(just(';').padded().or_not()).or_not() // Final expression with optional semicolon
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
            }
            else {
                DatexExpression::Statements(statements)
            }
        })
        .boxed();

    // primitive values (e.g. 1, "text", true, null)
    let integer = integer();
    let decimal = decimal();
    let text = text();
    let boolean = boolean();
    let null = null();
    let variable = variable();
    let placeholder = just('?').to(DatexExpression::Placeholder).boxed();

    // expression wrapped in parentheses
    let wrapped_expression = statements
        .clone()
        .delimited_by(just('('), just(')'))
        .padded();

    // a valid object/tuple key
    // (1: value), "key", 1, (("x"+"y"): 123)
    let key = choice((
        text.clone(),
        decimal.clone(),
        integer.clone(),
        // any valid identifiers (equivalent to variable names), mapped to a text
        text::ident()
            .map(|s: &str| DatexExpression::Text(s.to_string())),
        // dynamic key
        wrapped_expression.clone(),
    ));

    // array
    // 1,2,3
    // [1,2,3,4,13434,(1),4,5,7,8]
    let array = expression_without_tuple
        .clone()
        .separated_by(just(',').padded())
        .at_least(0)
        .allow_trailing()
        .collect()
        .padded()
        .delimited_by(just('['), just(']'))
        .map(DatexExpression::Array);

    // object
    let object = key.clone()
        .then_ignore(just(':').padded())
        .then(expression_without_tuple.clone())
        .separated_by(just(',').padded())
        .at_least(0)
        .allow_trailing()
        .collect()
        .padded()
        .delimited_by(just('{'), just('}'))
        .map(DatexExpression::Object);

    // tuple
    // Key-value pair
    let tuple_key_value_pair =
        key
            .clone()
            .then_ignore(just(':').padded())
            .then(expression_without_tuple.clone())
            .map(|(key, value)| TupleEntry::KeyValue(key, value));

    // tuple (either key:value entries or just values)
    let tuple_entry = choice((
        // Key-value pair
        tuple_key_value_pair.clone(),

        // Just a value with no key
        expression_without_tuple
            .clone()
            .map(TupleEntry::Value),
    )).boxed();

    let tuple = tuple_entry
        .clone()
        .separated_by(just(',').padded())
        .at_least(2)
        .collect::<Vec<_>>()
        .map(DatexExpression::Tuple);

    // e.g. x,
    let single_value_tuple = tuple_entry
        .clone()
        .then_ignore(just(',').padded())
        .map(|value| vec![value])
        .map(DatexExpression::Tuple);

    // e.g. (a:1)
    let single_keyed_tuple_entry = tuple_key_value_pair
        .clone()
        .map(|value| vec![value])
        .map(DatexExpression::Tuple);

    let tuple = choice((
        tuple,
        single_value_tuple,
        single_keyed_tuple_entry,
    ));

    // atomic expression (e.g. 1, "text", (1 + 2), (1;2))
    let atom = choice((
        array.clone(),
        object.clone(),
        placeholder,
        null,
        boolean,
        decimal.clone(),
        integer.clone(),
        text.clone(),
        variable.clone(),
        wrapped_expression.clone()
    )).boxed();

    // operations on atoms
    let op = |c| just(c).padded();

    // apply chain: two expressions following each other directly, optionally separated with "." (property access)
    let apply_or_property_access = atom.clone().then(
        choice((
            // apply #1: a wrapped expression, array, or object - no whitespace required before
            // x () x [] x {}
            choice((
                wrapped_expression.clone(),
                array.clone(),
                object.clone()
            ))
                .clone()
                .padded()
                .map(Apply::FunctionCall),
            // apply #2: an atomic value (e.g. "text") - whitespace or newline required before
            // print "sdf"
            one_of(" \n")
                .ignore_then(atom.clone().padded())
                .map(Apply::FunctionCall),
            // property access
            just('.').padded().ignore_then(key.clone())
                .map(Apply::PropertyAccess),
        ))
            .repeated()
            .collect::<Vec<_>>()
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
            op("*").to(binary_op(BinaryOperator::Multiply)),
            op("/").to(binary_op(BinaryOperator::Divide)),
        ))
            .then(apply_or_property_access)
            .repeated(),
        |lhs, (op, rhs)| op(Box::new(lhs), Box::new(rhs)),
    );

    let sum = product.clone().foldl(
        choice((
            op("+").to(binary_op(BinaryOperator::Add)),
            op("-").to(binary_op(BinaryOperator::Subtract)),
        ))
            .then(product)
            .repeated(),
        |lhs, (op, rhs)| op(Box::new(lhs), Box::new(rhs)),
    );

    // variable declarations or assignments
    let variable_assignment = just("val")
        .or(just("ref"))
        .or_not()
        .padded()
        .then::<&str, _>(text::ident())
        .then_ignore(just('=').padded())
        .then(sum.clone())
        .map(|((var_type, var_name), expr)| {
            if let Some(var_type) = var_type {
                DatexExpression::VariableDeclaration(
                    if var_type == "val" { VariableType::Value } else { VariableType::Reference },
                    var_name.to_string(),
                    Box::new(expr),
                )
            }
            else {
                DatexExpression::VariableAssignment(var_name.to_string(), Box::new(expr))
            }
        });

    expression_without_tuple.define(choice((
        variable_assignment,
        sum.clone(),
    )));

    expression.define(choice((
        tuple.clone(),
        expression_without_tuple.clone(),
    )).padded());

    choice((
        // empty script (0-n semicolons)
        just(';').repeated().at_least(1).padded().map(|_| DatexExpression::Statements(vec![])),
        // statements
        statements,
    )).boxed()
}


pub fn parse<'a>(src: &'a str, opt_parser: Option<&DatexScriptParser<'a>>) -> (Option<DatexExpression>, Vec<Rich<'a, char>>) {
    if let Some(parser) = opt_parser {
        // Use the provided parser
        let (res, errs) = parser.parse(src).into_output_errors();
        (res, errs)
    }
    else {
        let (res, errs) = create_parser().parse(src).into_output_errors();
        (res, errs)
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use ariadne::{Color, Label, Report, ReportKind, Source};

    fn print_report(errs: Vec<Rich<char>>, src: &str) {
        errs.into_iter().for_each(|e| {
            Report::build(ReportKind::Error, ((), e.span().into_range()))
                .with_config(ariadne::Config::new().with_index_type(ariadne::IndexType::Byte))
                .with_message(e.to_string())
                .with_label(
                    Label::new(((), e.span().into_range()))
                        .with_message(e.reason().to_string())
                        .with_color(Color::Red),
                )
                .finish()
                .eprint(Source::from(&src))
                .unwrap()
        });
    }

    fn parse_unwrap(src: &str) -> DatexExpression {
        let (res, errs) = parse(src, None);
        println!("{res:#?}");
        if !errs.is_empty() {
            print_report(errs, src);
            panic!("Parsing errors found");
        }
        res.unwrap()
    }

    fn try_parse_to_value_container(src: &str) -> ValueContainer {
        let expr = parse_unwrap(src);
        ValueContainer::try_from(expr).unwrap_or_else(|_| panic!("Failed to convert expression to ValueContainer"))
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

        assert_eq!(json, DatexExpression::Object(
            vec![
                (DatexExpression::Text("name".to_string()), DatexExpression::Text("Test".to_string())),
                (DatexExpression::Text("value".to_string()), DatexExpression::Integer(Integer::from(42))),
                (DatexExpression::Text("active".to_string()), DatexExpression::Boolean(true)),
                (DatexExpression::Text("items".to_string()), DatexExpression::Array(vec![
                    DatexExpression::Integer(Integer::from(1)),
                    DatexExpression::Integer(Integer::from(2)),
                    DatexExpression::Integer(Integer::from(3)),
                    DatexExpression::Decimal(Decimal::from_string("0.5"))
                ])),
                (DatexExpression::Text("nested".to_string()), DatexExpression::Object(
                    vec![(DatexExpression::Text("key".to_string()), DatexExpression::Text("value".to_string()))]
                        .into_iter().collect()
                )),
            ]
        ));
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
        assert_eq!(num, DatexExpression::Integer(Integer::from_string("123456789123456789").unwrap()));
    }

    #[test]
    fn test_negative_integer() {
        let src = "-123456789123456789";
        let num = parse_unwrap(src);
        assert_eq!(num, DatexExpression::Integer(Integer::from_string("-123456789123456789").unwrap()));
    }

    #[test]
    fn test_integer_with_underscores() {
        let src = "123_456";
        let num = parse_unwrap(src);
        assert_eq!(num, DatexExpression::Integer(Integer::from_string("123456").unwrap()));
    }

    #[test]
    fn test_hex_integer() {
        let src = "0x1A2B3C4D5E6F";
        let num = parse_unwrap(src);
        assert_eq!(num, DatexExpression::Integer(Integer::from_string_radix("1A2B3C4D5E6F", 16).unwrap()));
    }

    #[test]
    fn test_octal_integer() {
        let src = "0o755";
        let num = parse_unwrap(src);
        assert_eq!(num, DatexExpression::Integer(Integer::from_string_radix("755", 8).unwrap()));
    }

    #[test]
    fn test_binary_integer() {
        let src = "0b101010";
        let num = parse_unwrap(src);
        assert_eq!(num, DatexExpression::Integer(Integer::from_string_radix("101010", 2).unwrap()));
    }

    #[test]
    fn test_integer_with_exponent() {
        let src = "2e10";
        let num = parse_unwrap(src);
        assert_eq!(num, DatexExpression::Decimal(Decimal::from_string("20000000000")));
    }

    #[test]
    fn test_decimal() {
        let src = "123.456789123456";
        let num = parse_unwrap(src);
        assert_eq!(num, DatexExpression::Decimal(Decimal::from_string("123.456789123456")));
    }

    #[test]
    fn test_negative_decimal() {
        let src = "-123.4";
        let num = parse_unwrap(src);
        assert_eq!(num, DatexExpression::Decimal(Decimal::from_string("-123.4")));
    }

    #[test]
    fn test_decimal_with_exponent() {
        let src = "1.23456789123456e2";
        let num = parse_unwrap(src);
        assert_eq!(num, DatexExpression::Decimal(Decimal::from_string("123.456789123456")));
    }

    #[test]
    fn test_decimal_with_negative_exponent() {
        let src = "1.23456789123456e-2";
        let num = parse_unwrap(src);
        assert_eq!(num, DatexExpression::Decimal(Decimal::from_string("0.0123456789123456")));
    }

    #[test]
    fn test_decimal_with_positive_exponent() {
        let src = "1.23456789123456E+2";
        let num = parse_unwrap(src);
        assert_eq!(num, DatexExpression::Decimal(Decimal::from_string("123.456789123456")));
    }

    // TODO
    // #[test]
    // fn test_decimal_with_trailing_point() {
    //     let src = "123.";
    //     let num = try_parse(src);
    //     assert_eq!(num, DatexExpression::Decimal(Decimal::from_string("123.456789123456")));
    // }
    //
    // #[test]
    // fn test_decimal_with_leading_point() {
    //     let src = ".456789123456";
    //     let num = try_parse(src);
    //     assert_eq!(num, DatexExpression::Decimal(Decimal::from_string("0.456789123456")));
    // }
    //


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
        let src = r#""Hello, \"world\"! \n New line \t tab \uD83D\uDE00""#;
        let text = parse_unwrap(src);

        assert_eq!(text, DatexExpression::Text("Hello, \"world\"! \n New line \t tab 😀".to_string()));
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

        assert_eq!(arr, DatexExpression::Array(vec![
            DatexExpression::Integer(Integer::from(1)),
            DatexExpression::Integer(Integer::from(2)),
            DatexExpression::Integer(Integer::from(3)),
            DatexExpression::Decimal(Decimal::from_string("4.5")),
            DatexExpression::Text("text".to_string()),
        ]));
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

        assert_eq!(tuple, DatexExpression::Tuple(vec![
            TupleEntry::Value(DatexExpression::Integer(Integer::from(1))),
            TupleEntry::Value(DatexExpression::Integer(Integer::from(2))),
        ]));
    }

    #[test]
    fn test_scoped_tuple() {
        let src = "(1, 2)";
        let tuple = parse_unwrap(src);

        assert_eq!(tuple, DatexExpression::Tuple(vec![
            TupleEntry::Value(DatexExpression::Integer(Integer::from(1))),
            TupleEntry::Value(DatexExpression::Integer(Integer::from(2))),
        ]));
    }

    #[test]
    fn test_keyed_tuple() {
        let src = "1: 2, 3: 4, xy:2, 'a b c': 'd'";
        let tuple = parse_unwrap(src);

        assert_eq!(tuple, DatexExpression::Tuple(vec![
            TupleEntry::KeyValue(DatexExpression::Integer(Integer::from(1)), DatexExpression::Integer(Integer::from(2))),
            TupleEntry::KeyValue(DatexExpression::Integer(Integer::from(3)), DatexExpression::Integer(Integer::from(4))),
            TupleEntry::KeyValue(DatexExpression::Text("xy".to_string()), DatexExpression::Integer(Integer::from(2))),
            TupleEntry::KeyValue(DatexExpression::Text("a b c".to_string()), DatexExpression::Text("d".to_string())),
        ]));
    }

    #[test]
    fn test_tuple_array() {
        let src = "[(1,2),3,(4,)]";
        let arr = parse_unwrap(src);

        assert_eq!(arr, DatexExpression::Array(vec![
            DatexExpression::Tuple(vec![
                TupleEntry::Value(DatexExpression::Integer(Integer::from(1))),
                TupleEntry::Value(DatexExpression::Integer(Integer::from(2))),
            ]),
            DatexExpression::Integer(Integer::from(3)),
            DatexExpression::Tuple(vec![
                TupleEntry::Value(DatexExpression::Integer(Integer::from(4))),
            ]),
        ]));
    }

    #[test]
    fn test_single_value_tuple() {
        let src = "1,";
        let tuple = parse_unwrap(src);

        assert_eq!(tuple, DatexExpression::Tuple(vec![
            TupleEntry::Value(DatexExpression::Integer(Integer::from(1))),
        ]));
    }

    #[test]
    fn test_single_key_value_tuple() {
        let src = "x: 1";
        let tuple = parse_unwrap(src);
        assert_eq!(tuple, DatexExpression::Tuple(vec![
            TupleEntry::KeyValue(DatexExpression::Text("x".to_string()), DatexExpression::Integer(Integer::from(1))),
        ]));
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

        assert_eq!(arr, DatexExpression::Array(vec![
            DatexExpression::Integer(Integer::from(1)),
            DatexExpression::Integer(Integer::from(2)),
            DatexExpression::Integer(Integer::from(3)),
        ]));
    }

    #[test]
    fn test_object_with_key_value_pairs() {
        let src = r#"{"key1": "value1", "key2": 42, "key3": true}"#;
        let obj = parse_unwrap(src);

        assert_eq!(obj, DatexExpression::Object(vec![
            (DatexExpression::Text("key1".to_string()), DatexExpression::Text("value1".to_string())),
            (DatexExpression::Text("key2".to_string()), DatexExpression::Integer(Integer::from(42))),
            (DatexExpression::Text("key3".to_string()), DatexExpression::Boolean(true)),
        ]));
    }

    #[test]
    fn test_dynamic_object_keys() {
        let src = r#"{(1): "value1", (2): 42, (3): true}"#;
        let obj = parse_unwrap(src);
        assert_eq!(obj, DatexExpression::Object(vec![
            (DatexExpression::Integer(Integer::from(1)), DatexExpression::Text("value1".to_string())),
            (DatexExpression::Integer(Integer::from(2)), DatexExpression::Integer(Integer::from(42))),
            (DatexExpression::Integer(Integer::from(3)), DatexExpression::Boolean(true)),
        ]));
    }

    #[test]
    fn test_dynamic_tuple_keys() {
        let src = "(1): 1, ([]): 2";
        let tuple = parse_unwrap(src);

        assert_eq!(tuple, DatexExpression::Tuple(vec![
            TupleEntry::KeyValue(DatexExpression::Integer(Integer::from(1)), DatexExpression::Integer(Integer::from(1))),
            TupleEntry::KeyValue(DatexExpression::Array(vec![]), DatexExpression::Integer(Integer::from(2))),
        ]));
    }

    #[test]
    fn test_add() {
        // Test with escaped characters in text
        let src = "1 + 2";
        let expr = parse_unwrap(src);
        assert_eq!(expr, DatexExpression::BinaryOperation(
            BinaryOperator::Add,
            Box::new(DatexExpression::Integer(Integer::from(1))),
            Box::new(DatexExpression::Integer(Integer::from(2))),
        ));
    }

    #[test]
    fn test_add_complex_values() {
        // Test with escaped characters in text
        let src = "[] + x + (1 + 2)";
        let expr = parse_unwrap(src);
        assert_eq!(expr, DatexExpression::BinaryOperation(
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
        ));
    }

    #[test]
    fn test_subtract() {
        let src = "5 - 3";
        let expr = parse_unwrap(src);
        assert_eq!(expr, DatexExpression::BinaryOperation(
            BinaryOperator::Subtract,
            Box::new(DatexExpression::Integer(Integer::from(5))),
            Box::new(DatexExpression::Integer(Integer::from(3))),
        ));
    }

    #[test]
    fn test_multiply() {
        let src = "4 * 2";
        let expr = parse_unwrap(src);
        assert_eq!(expr, DatexExpression::BinaryOperation(
            BinaryOperator::Multiply,
            Box::new(DatexExpression::Integer(Integer::from(4))),
            Box::new(DatexExpression::Integer(Integer::from(2))),
        ));
    }

    #[test]
    fn test_divide() {
        let src = "8 / 2";
        let expr = parse_unwrap(src);
        assert_eq!(expr, DatexExpression::BinaryOperation(
            BinaryOperator::Divide,
            Box::new(DatexExpression::Integer(Integer::from(8))),
            Box::new(DatexExpression::Integer(Integer::from(2))),
        ));
    }

    #[test]
    fn test_complex_calculation() {
        let src = "1 + 2 * 3 + 4";
        let expr = parse_unwrap(src);
        assert_eq!(expr, DatexExpression::BinaryOperation(
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
        ));
    }

    #[test]
    fn test_nested_addition() {
        let src = "1 + (2 + 3)";
        let expr = parse_unwrap(src);
        assert_eq!(expr, DatexExpression::BinaryOperation(
            BinaryOperator::Add,
            Box::new(DatexExpression::Integer(Integer::from(1))),
            Box::new(DatexExpression::BinaryOperation(
                BinaryOperator::Add,
                Box::new(DatexExpression::Integer(Integer::from(2))),
                Box::new(DatexExpression::Integer(Integer::from(3))),
            )),
        ));
    }

    #[test]
    fn test_add_statements_1() {
        // Test with escaped characters in text
        let src = "1 + (2;3)";
        let expr = parse_unwrap(src);
        assert_eq!(expr, DatexExpression::BinaryOperation(
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
        ));
    }

    #[test]
    fn test_add_statements_2() {
        // Test with escaped characters in text
        let src = "(1;2) + 3";
        let expr = parse_unwrap(src);
        assert_eq!(expr, DatexExpression::BinaryOperation(
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
        ));
    }

    #[test]
    fn test_nested_expressions() {
        let src = "[1 + 2]";
        let expr = parse_unwrap(src);
        assert_eq!(expr, DatexExpression::Array(vec![
            DatexExpression::BinaryOperation(
                BinaryOperator::Add,
                Box::new(DatexExpression::Integer(Integer::from(1))),
                Box::new(DatexExpression::Integer(Integer::from(2))),
            ),
        ]));
    }

    #[test]
    fn multi_statement_expression() {
        let src = "1;2";
        let expr = parse_unwrap(src);
        assert_eq!(expr, DatexExpression::Statements(vec![
            Statement {
                expression: DatexExpression::Integer(Integer::from(1)),
                is_terminated: true,
            },
            Statement {
                expression: DatexExpression::Integer(Integer::from(2)),
                is_terminated: false,
            },
        ]));
    }

    #[test]
    fn nested_scope_statements() {
        let src = "(1; 2; 3)";
        let expr = parse_unwrap(src);
        assert_eq!(expr, DatexExpression::Statements(vec![
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
        ]));
    }
    #[test]
    fn nested_scope_statements_closed() {
        let src = "(1; 2; 3;)";
        let expr = parse_unwrap(src);
        assert_eq!(expr, DatexExpression::Statements(vec![
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
        ]));
    }

    #[test]
    fn nested_statements_in_object() {
        let src = r#"{"key": (1; 2; 3)}"#;
        let expr = parse_unwrap(src);
        assert_eq!(expr, DatexExpression::Object(vec![
            (DatexExpression::Text("key".to_string()), DatexExpression::Statements(vec![
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
            ])),
        ]));
    }

    #[test]
    fn test_single_statement() {
        let src = "1;";
        let expr = parse_unwrap(src);
        assert_eq!(expr, DatexExpression::Statements(vec![
            Statement {
                expression: DatexExpression::Integer(Integer::from(1)),
                is_terminated: true,
            },
        ]));
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
        assert_eq!(expr, DatexExpression::BinaryOperation(
            BinaryOperator::Add,
            Box::new(DatexExpression::Variable("myVar".to_string())),
            Box::new(DatexExpression::Integer(Integer::from(1))),
        ));
    }

    #[test]
    fn test_apply_expression() {
        let src = "myFunc(1, 2, 3)";
        let expr = parse_unwrap(src);
        assert_eq!(expr, DatexExpression::ApplyChain(
            Box::new(DatexExpression::Variable("myFunc".to_string())),
            vec![
                Apply::FunctionCall(
                    DatexExpression::Tuple(vec![
                        TupleEntry::Value(DatexExpression::Integer(Integer::from(1))),
                        TupleEntry::Value(DatexExpression::Integer(Integer::from(2))),
                        TupleEntry::Value(DatexExpression::Integer(Integer::from(3))),
                    ]),
                )
            ],
        ));
    }

    #[test]
    fn test_apply_empty() {
        let src = "myFunc()";
        let expr = parse_unwrap(src);
        assert_eq!(expr, DatexExpression::ApplyChain(
            Box::new(DatexExpression::Variable("myFunc".to_string())),
            vec![Apply::FunctionCall(DatexExpression::Statements(vec![]))],
        ));
    }

    #[test]
    fn test_apply_multiple() {
        let src = "myFunc(1)(2, 3)";
        let expr = parse_unwrap(src);
        assert_eq!(expr, DatexExpression::ApplyChain(
            Box::new(DatexExpression::Variable("myFunc".to_string())),
            vec![
                Apply::FunctionCall(
                    DatexExpression::Integer(Integer::from(1)),
                ),
                Apply::FunctionCall(
                    DatexExpression::Tuple(vec![
                        TupleEntry::Value(DatexExpression::Integer(Integer::from(2))),
                        TupleEntry::Value(DatexExpression::Integer(Integer::from(3))),
                    ])
                )
            ],
        ));
    }

    #[test]
    fn test_apply_atom() {
        let src = "print 'test'";
        let expr = parse_unwrap(src);
        assert_eq!(expr, DatexExpression::ApplyChain(
            Box::new(DatexExpression::Variable("print".to_string())),
            vec![
                Apply::FunctionCall(DatexExpression::Text("test".to_string()))
            ],
        ));
    }

    #[test]
    fn test_property_access() {
        let src = "myObj.myProp";
        let expr = parse_unwrap(src);
        assert_eq!(expr, DatexExpression::ApplyChain(
            Box::new(DatexExpression::Variable("myObj".to_string())),
            vec![Apply::PropertyAccess(DatexExpression::Text("myProp".to_string()))],
        ));
    }

    #[test]
    fn test_property_access_scoped() {
        let src = "myObj.(1)";
        let expr = parse_unwrap(src);
        assert_eq!(expr, DatexExpression::ApplyChain(
            Box::new(DatexExpression::Variable("myObj".to_string())),
            vec![Apply::PropertyAccess(DatexExpression::Integer(Integer::from(1)))],
        ));
    }

    #[test]
    fn test_property_access_multiple() {
        let src = "myObj.myProp.anotherProp.(1 + 2).(x;y)";
        let expr = parse_unwrap(src);
        assert_eq!(expr, DatexExpression::ApplyChain(
            Box::new(DatexExpression::Variable("myObj".to_string())),
            vec![
                Apply::PropertyAccess(DatexExpression::Text("myProp".to_string())),
                Apply::PropertyAccess(DatexExpression::Text("anotherProp".to_string())),
                Apply::PropertyAccess(DatexExpression::BinaryOperation(
                    BinaryOperator::Add,
                    Box::new(DatexExpression::Integer(Integer::from(1))),
                    Box::new(DatexExpression::Integer(Integer::from(2))),
                )),
                Apply::PropertyAccess(DatexExpression::Statements(vec![
                    Statement {
                        expression: DatexExpression::Variable("x".to_string()),
                        is_terminated: true,
                    },
                    Statement {
                        expression: DatexExpression::Variable("y".to_string()),
                        is_terminated: false,
                    },
                ])),
            ],
        ));
    }

    #[test]
    fn test_property_access_and_apply() {
        let src = "myObj.myProp(1, 2)";
        let expr = parse_unwrap(src);
        assert_eq!(expr, DatexExpression::ApplyChain(
            Box::new(DatexExpression::Variable("myObj".to_string())),
            vec![
                Apply::PropertyAccess(DatexExpression::Text("myProp".to_string())),
                Apply::FunctionCall(DatexExpression::Tuple(vec![
                    TupleEntry::Value(DatexExpression::Integer(Integer::from(1))),
                    TupleEntry::Value(DatexExpression::Integer(Integer::from(2))),
                ])),
            ],
        ));
    }

    #[test]
    fn test_apply_and_property_access() {
        let src = "myFunc(1).myProp";
        let expr = parse_unwrap(src);
        assert_eq!(expr, DatexExpression::ApplyChain(
            Box::new(DatexExpression::Variable("myFunc".to_string())),
            vec![
                Apply::FunctionCall(DatexExpression::Integer(Integer::from(1))),
                Apply::PropertyAccess(DatexExpression::Text("myProp".to_string())),
            ],
        ));
    }

    #[test]
    fn nested_apply_and_property_access() {
        let src = "((x(1)).y).z";
        let expr = parse_unwrap(src);
        assert_eq!(expr, DatexExpression::ApplyChain(
            Box::new(DatexExpression::ApplyChain(
                Box::new(DatexExpression::ApplyChain(
                    Box::new(DatexExpression::Variable("x".to_string())),
                    vec![Apply::FunctionCall(DatexExpression::Integer(Integer::from(1)))],
                )),
                vec![Apply::PropertyAccess(DatexExpression::Text("y".to_string()))],
            )),
            vec![Apply::PropertyAccess(DatexExpression::Text("z".to_string()))],
        ));
    }

    #[test]
    fn variable_declaration() {
        let src = "val x = 42";
        let expr = parse_unwrap(src);
        assert_eq!(expr, DatexExpression::VariableDeclaration(
            VariableType::Value,
            "x".to_string(),
            Box::new(DatexExpression::Integer(Integer::from(42))),
        ));
    }

    #[test]
    fn variable_declaration_statement() {
        let src = "val x = 42;";
        let expr = parse_unwrap(src);
        assert_eq!(expr, DatexExpression::Statements(vec![
            Statement {
                expression: DatexExpression::VariableDeclaration(
                    VariableType::Value,
                    "x".to_string(),
                    Box::new(DatexExpression::Integer(Integer::from(42))),
                ),
                is_terminated: true,
            },
        ]));
    }

    #[test]
    fn variable_declaration_with_expression() {
        let src = "ref x = 1 + 2";
        let expr = parse_unwrap(src);
        assert_eq!(expr, DatexExpression::VariableDeclaration(
            VariableType::Reference,
            "x".to_string(),
            Box::new(DatexExpression::BinaryOperation(
                BinaryOperator::Add,
                Box::new(DatexExpression::Integer(Integer::from(1))),
                Box::new(DatexExpression::Integer(Integer::from(2))),
            )),
        ));
    }

    #[test]
    fn variable_assignment() {
        let src = "x = 42";
        let expr = parse_unwrap(src);
        assert_eq!(expr, DatexExpression::VariableAssignment(
            "x".to_string(),
            Box::new(DatexExpression::Integer(Integer::from(42))),
        ));
    }

    #[test]
    fn variable_assignment_expression() {
        let src = "x = (y = 1)";
        let expr = parse_unwrap(src);
        assert_eq!(expr, DatexExpression::VariableAssignment(
            "x".to_string(),
            Box::new(DatexExpression::VariableAssignment(
                "y".to_string(),
                Box::new(DatexExpression::Integer(Integer::from(1))),
            )),
        ));
    }

    #[test]
    fn variable_assignment_expression_in_array() {
        let src = "[x = 1]";
        let expr = parse_unwrap(src);
        assert_eq!(expr, DatexExpression::Array(vec![
            DatexExpression::VariableAssignment(
                "x".to_string(),
                Box::new(DatexExpression::Integer(Integer::from(1))),
            ),
        ]));
    }

    #[test]
    fn apply_in_array() {
        let src = "[myFunc(1)]";
        let expr = parse_unwrap(src);
        assert_eq!(expr, DatexExpression::Array(vec![
            DatexExpression::ApplyChain(
                Box::new(DatexExpression::Variable("myFunc".to_string())),
                vec![Apply::FunctionCall(DatexExpression::Integer(Integer::from(1)))]
            ),
        ]));
    }

    // TODO:
    // #[test]
    // fn variable_assignment_multiple() {
    //     let src = "x = y = 42";
    //     let expr = parse_unwrap(src);
    //     assert_eq!(expr, DatexExpression::VariableAssignment(
    //         "x".to_string(),
    //         Box::new(DatexExpression::VariableAssignment(
    //             "y".to_string(),
    //             Box::new(DatexExpression::Integer(Integer::from(42))),
    //         )),
    //     ));
    // }

    #[test]
    fn variable_declaration_and_assignment() {
        let src = "val x = 42; x = 100 * 10;";
        let expr = parse_unwrap(src);
        assert_eq!(expr, DatexExpression::Statements(vec![
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
                        Box::new(DatexExpression::Integer(Integer::from(100))),
                        Box::new(DatexExpression::Integer(Integer::from(10))),
                    )),
                ),
                is_terminated: true,
            },
        ]));
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
        assert_eq!(val, ValueContainer::from(Integer::from_string("123456789123456789").unwrap()));
    }

    #[test]
    fn test_decimal_to_value_container() {
        let src = "123.456789123456";
        let val = try_parse_to_value_container(src);
        assert_eq!(val, ValueContainer::from(Decimal::from_string("123.456789123456")));
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
        let value_container_inner_object: ValueContainer = ValueContainer::from(
            Object::from(vec![
                ("key".to_string(), "value".to_string().into()),
            ].into_iter().collect::<HashMap<String, ValueContainer>>()
            )
        );
        let value_container_object: ValueContainer = ValueContainer::from(
            Object::from(vec![
                ("name".to_string(), "Test".to_string().into()),
                ("value".to_string(), Integer::from(42).into()),
                ("active".to_string(), true.into()),
                ("items".to_string(), value_container_array.into()),
                ("nested".to_string(), value_container_inner_object),
            ].into_iter().collect::<HashMap<String, ValueContainer>>())
        );
        assert_eq!(val, value_container_object);
    }
    #[test]
    fn test_invalid_value_containers() {
        let src = "1 + 2";
        let expr = parse_unwrap(src);
        assert!(ValueContainer::try_from(expr).is_err(), "Expected error when converting expression to ValueContainer");

        let src = "xy";
        let expr = parse_unwrap(src);
        assert!(ValueContainer::try_from(expr).is_err(), "Expected error when converting expression to ValueContainer");

        let src = "x()";
        let expr = parse_unwrap(src);
        assert!(ValueContainer::try_from(expr).is_err(), "Expected error when converting expression to ValueContainer");
    }

    #[test]
    fn test_invalid_add() {
        let src = "1+2";
        let (res, errs) = parse(src, None);
        assert!(errs.len() == 1, "Expected error when parsing expression");
    }
}