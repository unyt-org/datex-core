use crate::compiler::parser_new::extra::Err;
use chumsky::prelude::*;
use chumsky::recursive::Indirect;
use crate::datex_values::core_values::decimal::decimal::Decimal;
use crate::datex_values::core_values::integer::integer::Integer;

#[derive(Clone, Debug, PartialEq)]
pub enum TupleEntry {
    KeyValue(DatexExpression, DatexExpression),
    ValueOnly(DatexExpression),
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
pub enum DatexExpression {
    /// Invalid expression, e.g. syntax error
    Invalid,

    /// null
    Null,
    /// Boolean (true or false)
    Bool(bool),
    /// Text, e.g "Hello, world!"
    Text(String),
    /// Decimal, e.g 123.456789123456
    Decimal(Decimal),
    /// Integer, e.g 123456789123456789
    Integer(Integer),
    /// Array, e.g [1, 2, 3, "text"]
    Array(Vec<DatexExpression>),
    /// Object, e.g {"key": "value", key2: 2}
    Object(Vec<(DatexExpression, DatexExpression)>),
    /// Tuple, e.g (1: 2, 3: 4, "xy") or without brackets: 1,2,a:3
    Tuple(Vec<TupleEntry>),
    /// One or more statements, e.g (1; 2; 3)
    Statements(Vec<Statement>),
    /// Identifier, e.g. a variable name
    Variable(String),

    BinaryOperation(BinaryOperator, Box<DatexExpression>, Box<DatexExpression>),
    UnaryOperation(UnaryOperator, Box<DatexExpression>),
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

type DatexExpressionParser<'a> = Boxed<'a, 'a, &'a str, DatexExpression, Err<Rich<'a, char>>>;

fn text<'a>() -> DatexExpressionParser<'a> {
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
        )))
        .boxed();

    let text_double_quotes = none_of("\\\"")
        .or(escape.clone())
        .repeated()
        .collect::<String>()
        .delimited_by(just('"'), just('"'))
        .boxed();

    let text_single_quotes = none_of("\\'")
        .or(escape)
        .repeated()
        .collect::<String>()
        .delimited_by(just('\''), just('\''))
        .boxed();


    let text = choice((
        text_double_quotes.clone().map(DatexExpression::Text),
        text_single_quotes.clone().map(DatexExpression::Text),
    )).boxed();

    text
}

fn integer<'a>() -> DatexExpressionParser<'a> {
    let digits = text::digits(10).to_slice();
    let integer = digits
        .map(|s: &str| Integer::from_string(s).unwrap())
        .map(DatexExpression::Integer)
        .boxed();
    integer
}

fn decimal<'a>() -> DatexExpressionParser<'a> {
    let digits = text::digits(10).to_slice();
    let frac = just('.').then(digits);
    let exp = just('e')
        .or(just('E'))
        .then(one_of("+-").or_not())
        .then(digits);

    let decimal = just('-')
        .or_not()
        .then(text::int(10))
        .then(frac)
        .then(exp.or_not())
        .to_slice()
        .map(|s: &str| Decimal::from_string(s))
        .map(DatexExpression::Decimal)
        .boxed();

    decimal
}

fn boolean<'a>() -> DatexExpressionParser<'a> {
    let true_value = just("true").to(DatexExpression::Bool(true));
    let false_value = just("false").to(DatexExpression::Bool(false));

    let boolean = choice((true_value, false_value)).boxed();

    boolean
}

fn null<'a>() -> DatexExpressionParser<'a> {
    let null_value = just("null").to(DatexExpression::Null);
    null_value.boxed()
}

fn variable<'a>() -> DatexExpressionParser<'a> {
    // valid identifiers start with _ or an ascii letter, followed by any combination of letters, digits, or underscores
    let identifier = text::ident()
        .map(|s: &str| DatexExpression::Variable(s.to_string()))
        .boxed();
    identifier
}


fn binary_op(op: BinaryOperator) -> impl Fn(Box<DatexExpression>, Box<DatexExpression>) -> DatexExpression + Clone {
    move |lhs, rhs| DatexExpression::BinaryOperation(op.clone(), lhs, rhs)
}

/// Apply operations in the correct order on the datex expression parser
fn operations(expression: DatexExpressionParser) -> DatexExpressionParser {

    let op = |c| just(c).padded();

    let product = expression.clone().foldl(
        choice((
            op('*').to(binary_op(BinaryOperator::Multiply)),
            op('/').to(binary_op(BinaryOperator::Divide)),
        ))
            .then(expression)
            .repeated(),
        |lhs, (op, rhs)| op(Box::new(lhs), Box::new(rhs)),
    );

    let sum = product.clone().foldl(
        choice((
            op('+').to(binary_op(BinaryOperator::Add)),
            op('-').to(binary_op(BinaryOperator::Subtract)),
        ))
            .then(product)
            .repeated(),
        |lhs, (op, rhs)| op(Box::new(lhs), Box::new(rhs)),
    );

    sum.boxed()
}

fn parser<'a>() -> DatexExpressionParser<'a> {

    // an expression
    let mut expression = Recursive::declare();

    // an expression without tuple entries - required to be used inside arrays and objects to prevent matching tuples
    let mut expression_without_tuple = Recursive::declare();

    // atomic values (e.g. 1, "text", true, null)
    let integer = integer();
    let decimal = decimal();
    let text = text();
    let boolean = boolean();
    let null = null();
    let variable = variable();

    let atom = choice((
        null,
        boolean,
        decimal.clone(),
        integer.clone(),
        text.clone(),
        variable.clone(),
    )).boxed();

    // expression wrapped in parentheses
    let scoped_expression = expression
        .clone()
        .delimited_by(
            just('('),
            just(')')
                .ignored()
                .recover_with(via_parser(end()))
                .recover_with(skip_then_retry_until(any().ignored(), end())),
        )
        .boxed();

    // a valid object/tuple key
    let key = choice((
        text.clone(),
        decimal.clone(),
        integer.clone(),
        // any valid identifiers (equivalent to variable names), mapped to a text
        text::ident()
            .map(|s: &str| DatexExpression::Text(s.to_string()))
            .boxed(),
        scoped_expression.clone(),
    ));

    // array
    let array = expression_without_tuple
        .clone()
        .separated_by(just(',').padded().recover_with(skip_then_retry_until(
            any().ignored(),
            one_of(",]").ignored(),
        )))
        .at_least(0)
        .allow_trailing()
        .collect()
        .padded()
        .delimited_by(
            just('['),
            just(']')
                .ignored()
                .recover_with(via_parser(end()))
                .recover_with(skip_then_retry_until(any().ignored(), end())),
        )
        .boxed()
        .map(DatexExpression::Array);

    // object
    let object = key.clone()
        .then_ignore(just(':').padded())
        .then(expression_without_tuple.clone())
        .separated_by(just(',').padded().recover_with(skip_then_retry_until(
            any().ignored(),
            one_of(",}").ignored(),
        )))
        .at_least(0)
        .allow_trailing()
        .collect()
        .padded()
        .delimited_by(
            just('{'),
            just('}')
                .ignored()
                .recover_with(via_parser(end()))
                .recover_with(skip_then_retry_until(any().ignored(), end())),
        )
        .boxed()
        .map(DatexExpression::Object);


    // tuple (either key:value entries or just values)
    let tuple_entry = choice((
        // Key-value pair
        key
            .clone()
            .then_ignore(just(':').padded())
            .then(expression_without_tuple.clone())
            .map(|(key, value)| TupleEntry::KeyValue(key, value)),

        // Just a value with no key
        expression_without_tuple
            .clone()
            .map(TupleEntry::ValueOnly),
    )).boxed();

    let tuple = tuple_entry
        .separated_by(just(',').padded())
        .at_least(2)
        .collect::<Vec<_>>()
        .map(DatexExpression::Tuple)
        .boxed();

    let single_value_tuple = expression_without_tuple
        .clone()
        .then_ignore(just(',').padded())
        .map(|value| vec![TupleEntry::ValueOnly(value)])
        .map(DatexExpression::Tuple)
        .boxed();

    // an atomic expression, containing a single value, array, object, or tuple
    // a datex script source consists of a sequence of expressions
    let atomic_expression = choice((
        tuple.clone(),
        single_value_tuple.clone(),
        atom.clone(),
        scoped_expression.clone(),
        array.clone(),
        object.clone(),
    ))
        .recover_with(via_parser(nested_delimiters(
            '{',
            '}',
            [('[', ']'), ('(', ')')],
            |_| DatexExpression::Invalid,
        )))
        .recover_with(via_parser(nested_delimiters(
            '[',
            ']',
            [('{', '}'), ('(', ')')],
            |_| DatexExpression::Invalid,
        )))
        .recover_with(via_parser(nested_delimiters(
            '(',
            ')',
            [('{', '}'), ('[', ']')],
            |_| DatexExpression::Invalid,
        )))
        .recover_with(skip_then_retry_until(
            any().ignored(),
            one_of(",]}").ignored(),
        ))
        .padded().boxed();

    let x = operations(choice((
        atomic_expression.clone(),
    )).boxed());

    // statement: expression with an optional semicolon at the end
    let statement = x
        .clone()
        .map(|expr| Statement {
            expression: expr,
            is_terminated: false,
        })
        .or(x
            .clone()
            .then_ignore(just(';').padded())
            .map(|expr| Statement {
                expression: expr,
                is_terminated: true,
            }));

    // expression with semicolon
    let closed_statement = x.clone().then_ignore(just(';').padded())
        .map(|expr| Statement {
            expression: expr,
            is_terminated: true,
        });

    // multiple statements separated by semicolons
    // first statement must be closed, subsequent statements can be closed or not
    let statements = closed_statement
        .then(
            statement
                .repeated()
                .collect::<Vec<_>>()
        )
        .map(|(first, rest)| {
            let mut all_statements = vec![first];
            all_statements.extend(rest);
            DatexExpression::Statements(all_statements)
        })
        .boxed();

    // atomic expression wrapped with operations
    expression.define(
        choice((
            statements,
            x,
        ))
    );

    // TODO: make this better without duplicate definition and operations() call?!
    expression_without_tuple.define(
        operations(choice((
            atom,
            scoped_expression,
            array,
            object,
        )).boxed())
    );

    expression.boxed()
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

    fn try_parse(src: &str) -> DatexExpression {
        let (res, errs) = parser().parse(src).into_output_errors();
        println!("{res:#?}");
        if !errs.is_empty() {
            print_report(errs, src);
            panic!("Parsing errors found");
        }
        res.unwrap()
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

        let json = try_parse(src);

        assert_eq!(json, DatexExpression::Object(
            vec![
                (DatexExpression::Text("name".to_string()), DatexExpression::Text("Test".to_string())),
                (DatexExpression::Text("value".to_string()), DatexExpression::Integer(Integer::from(42))),
                (DatexExpression::Text("active".to_string()), DatexExpression::Bool(true)),
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
        let val = try_parse(src);
        assert_eq!(val, DatexExpression::Null);
    }

    #[test]
    fn test_boolean() {
        let src_true = "true";
        let val_true = try_parse(src_true);
        assert_eq!(val_true, DatexExpression::Bool(true));

        let src_false = "false";
        let val_false = try_parse(src_false);
        assert_eq!(val_false, DatexExpression::Bool(false));
    }

    #[test]
    fn test_integer() {
        let src = "123456789123456789";
        let num = try_parse(src);
        assert_eq!(num, DatexExpression::Integer(Integer::from_string("123456789123456789").unwrap()));
    }

    #[test]
    fn test_decimal() {
        let src = "123.456789123456";
        let num = try_parse(src);
        assert_eq!(num, DatexExpression::Decimal(Decimal::from_string("123.456789123456")));
    }

    #[test]
    fn test_decimal_with_exponent() {
        let src = "1.23456789123456e2";
        let num = try_parse(src);
        assert_eq!(num, DatexExpression::Decimal(Decimal::from_string("123.456789123456")));
    }

    #[test]
    fn test_text_double_quotes() {
        let src = r#""Hello, world!""#;
        let text = try_parse(src);
        assert_eq!(text, DatexExpression::Text("Hello, world!".to_string()));
    }

    #[test]
    fn test_text_single_quotes() {
        let src = r#"'Hello, world!'"#;
        let text = try_parse(src);
        assert_eq!(text, DatexExpression::Text("Hello, world!".to_string()));
    }

    #[test]
    fn test_text_escape_sequences() {
        let src = r#""Hello, \"world\"! \n New line \t tab \uD83D\uDE00""#;
        let text = try_parse(src);

        assert_eq!(text, DatexExpression::Text("Hello, \"world\"! \n New line \t tab 😀".to_string()));
    }


    #[test]
    fn test_empty_array() {
        let src = "[]";
        let arr = try_parse(src);
        assert_eq!(arr, DatexExpression::Array(vec![]));
    }

    #[test]
    fn test_array_with_values() {
        let src = "[1, 2, 3, 4.5, \"text\"]";
        let arr = try_parse(src);

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
        let obj = try_parse(src);

        assert_eq!(obj, DatexExpression::Object(vec![]));
    }

    #[test]
    fn test_tuple() {
        let src = "1,2";
        let tuple = try_parse(src);

        assert_eq!(tuple, DatexExpression::Tuple(vec![
            TupleEntry::ValueOnly(DatexExpression::Integer(Integer::from(1))),
            TupleEntry::ValueOnly(DatexExpression::Integer(Integer::from(2))),
        ]));
    }

    #[test]
    fn test_scoped_tuple() {
        let src = "(1, 2)";
        let tuple = try_parse(src);

        assert_eq!(tuple, DatexExpression::Tuple(vec![
            TupleEntry::ValueOnly(DatexExpression::Integer(Integer::from(1))),
            TupleEntry::ValueOnly(DatexExpression::Integer(Integer::from(2))),
        ]));
    }

    #[test]
    fn test_keyed_tuple() {
        let src = "1: 2, 3: 4, xy:2, 'a b c': 'd'";
        let tuple = try_parse(src);

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
        let arr = try_parse(src);

        assert_eq!(arr, DatexExpression::Array(vec![
            DatexExpression::Tuple(vec![
                TupleEntry::ValueOnly(DatexExpression::Integer(Integer::from(1))),
                TupleEntry::ValueOnly(DatexExpression::Integer(Integer::from(2))),
            ]),
            DatexExpression::Integer(Integer::from(3)),
            DatexExpression::Tuple(vec![
                TupleEntry::ValueOnly(DatexExpression::Integer(Integer::from(4))),
            ]),
        ]));
    }

    #[test]
    fn test_single_value_tuple() {
        let src = "1,";
        let tuple = try_parse(src);

        assert_eq!(tuple, DatexExpression::Tuple(vec![
            TupleEntry::ValueOnly(DatexExpression::Integer(Integer::from(1))),
        ]));
    }

    #[test]
    fn test_scoped_atom() {
        let src = "(1)";
        let atom = try_parse(src);
        assert_eq!(atom, DatexExpression::Integer(Integer::from(1)));
    }

    #[test]
    fn test_scoped_array() {
        let src = "(([1, 2, 3]))";
        let arr = try_parse(src);

        assert_eq!(arr, DatexExpression::Array(vec![
            DatexExpression::Integer(Integer::from(1)),
            DatexExpression::Integer(Integer::from(2)),
            DatexExpression::Integer(Integer::from(3)),
        ]));
    }

    #[test]
    fn test_object_with_key_value_pairs() {
        let src = r#"{"key1": "value1", "key2": 42, "key3": true}"#;
        let obj = try_parse(src);

        assert_eq!(obj, DatexExpression::Object(vec![
            (DatexExpression::Text("key1".to_string()), DatexExpression::Text("value1".to_string())),
            (DatexExpression::Text("key2".to_string()), DatexExpression::Integer(Integer::from(42))),
            (DatexExpression::Text("key3".to_string()), DatexExpression::Bool(true)),
        ]));
    }

    #[test]
    fn test_dynamic_object_keys() {
        let src = r#"{(1): "value1", (2): 42, (3): true}"#;
        let obj = try_parse(src);
        assert_eq!(obj, DatexExpression::Object(vec![
            (DatexExpression::Integer(Integer::from(1)), DatexExpression::Text("value1".to_string())),
            (DatexExpression::Integer(Integer::from(2)), DatexExpression::Integer(Integer::from(42))),
            (DatexExpression::Integer(Integer::from(3)), DatexExpression::Bool(true)),
        ]));
    }

    #[test]
    fn test_dynamic_tuple_keys() {
        let src = "(1): 1, ([]): 2";
        let tuple = try_parse(src);

        assert_eq!(tuple, DatexExpression::Tuple(vec![
            TupleEntry::KeyValue(DatexExpression::Integer(Integer::from(1)), DatexExpression::Integer(Integer::from(1))),
            TupleEntry::KeyValue(DatexExpression::Array(vec![]), DatexExpression::Integer(Integer::from(2))),
        ]));
    }

    #[test]
    fn test_add() {
        // Test with escaped characters in text
        let src = "1+2";
        let expr = try_parse(src);
        assert_eq!(expr, DatexExpression::BinaryOperation(
            BinaryOperator::Add,
            Box::new(DatexExpression::Integer(Integer::from(1))),
            Box::new(DatexExpression::Integer(Integer::from(2))),
        ));
    }

    #[test]
    fn test_subtract() {
        let src = "5 - 3";
        let expr = try_parse(src);
        assert_eq!(expr, DatexExpression::BinaryOperation(
            BinaryOperator::Subtract,
            Box::new(DatexExpression::Integer(Integer::from(5))),
            Box::new(DatexExpression::Integer(Integer::from(3))),
        ));
    }

    #[test]
    fn test_multiply() {
        let src = "4 * 2";
        let expr = try_parse(src);
        assert_eq!(expr, DatexExpression::BinaryOperation(
            BinaryOperator::Multiply,
            Box::new(DatexExpression::Integer(Integer::from(4))),
            Box::new(DatexExpression::Integer(Integer::from(2))),
        ));
    }

    #[test]
    fn test_divide() {
        let src = "8 / 2";
        let expr = try_parse(src);
        assert_eq!(expr, DatexExpression::BinaryOperation(
            BinaryOperator::Divide,
            Box::new(DatexExpression::Integer(Integer::from(8))),
            Box::new(DatexExpression::Integer(Integer::from(2))),
        ));
    }

    #[test]
    fn test_complex_calculation() {
        let src = "1 + 2 * 3 + 4";
        let expr = try_parse(src);
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
    fn test_nested_expressions() {
        let src = "[1 + 2]";
        let expr = try_parse(src);
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
        let expr = try_parse(src);
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

}