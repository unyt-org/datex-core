use crate::compiler::parser_new::extra::Err;
use chumsky::prelude::*;
use chumsky::extra::ParserExtra;
use crate::datex_values::core_values::decimal::decimal::Decimal;
use crate::datex_values::core_values::integer::integer::Integer;

#[derive(Clone, Debug, PartialEq)]
enum TupleEntry {
    KeyValue(DatexExpression, DatexExpression),
    ValueOnly(DatexExpression),
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
    /// Expression block, e.g (1; 2; 3)
    ExpressionBlock(Vec<DatexExpression>),
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

fn atom<'a>() -> DatexExpressionParser<'a> {
    // primitive values
    let digits = text::digits(10).to_slice();
    let frac = just('.').then(digits);
    let exp = just('e')
        .or(just('E'))
        .then(one_of("+-").or_not())
        .then(digits);

    let integer = digits
        .map(|s: &str| Integer::from_string(s).unwrap())
        .boxed();

    let decimal = just('-')
        .or_not()
        .then(text::int(10))
        .then(frac)
        .then(exp.or_not())
        .to_slice()
        .map(|s: &str| Decimal::from_string(s))
        .boxed();

    let text = text();

    let atom = choice((
        just("null").to(DatexExpression::Null),
        just("true").to(DatexExpression::Bool(true)),
        just("false").to(DatexExpression::Bool(false)),
        decimal.map(DatexExpression::Decimal),
        integer.map(DatexExpression::Integer),
        text.clone(),
    )).boxed();

    atom
}


fn parser<'a>() -> impl Parser<'a, &'a str, DatexExpression, extra::Err<Rich<'a, char>>> {

    // a generic expression
    // a datex script source consists of a sequence of expressions
    let mut expression = Recursive::declare();
    // an expression without tuple entries - required to be used inside arrays and objects to prevent matching tuples
    let mut expression_without_tuple = Recursive::declare();

    // an atomic value (e.g. 1, "text", true, null)
    let atom = atom();

    let text = text();

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
    let object = text.clone()
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
        text
            .clone()
            .then_ignore(just(':').padded())
            .then(expression.clone())
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

    expression_without_tuple.define(choice((
        atom,
        scoped_expression,
        array,
        object,
    )));

    // returns atom
    expression.define(choice((
        tuple,
        single_value_tuple,
        expression_without_tuple
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
        .padded()
    );

    expression
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

        let (json, errs) = parser().parse(src.trim()).into_output_errors();
        println!("{json:#?}");
        if !errs.is_empty() {
            print_report(errs, src);
            panic!("Parsing errors found");
        }

        assert_eq!(json.unwrap(), DatexExpression::Object(
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
        let (val, errs) = parser().parse(src).into_output_errors();
        println!("{val:#?}");
        if !errs.is_empty() {
            print_report(errs, src);
            panic!("Parsing errors found");
        }

        assert_eq!(val.unwrap(), DatexExpression::Null);
    }

    #[test]
    fn test_boolean() {
        let src_true = "true";
        let (val_true, errs_true) = parser().parse(src_true).into_output_errors();
        println!("{val_true:#?}");
        if !errs_true.is_empty() {
            print_report(errs_true, src_true);
            panic!("Parsing errors found");
        }
        assert_eq!(val_true.unwrap(), DatexExpression::Bool(true));

        let src_false = "false";
        let (val_false, errs_false) = parser().parse(src_false).into_output_errors();
        println!("{val_false:#?}");
        if !errs_false.is_empty() {
            print_report(errs_false, src_false);
            panic!("Parsing errors found");
        }
        assert_eq!(val_false.unwrap(), DatexExpression::Bool(false));
    }

    #[test]
    fn test_integer() {
        let src = "123456789123456789";
        let (num, errs) = parser().parse(src).into_output_errors();
        println!("{num:#?}");
        if !errs.is_empty() {
            print_report(errs, src);
            panic!("Parsing errors found");
        }

        assert_eq!(num.unwrap(), DatexExpression::Integer(Integer::from_string("123456789123456789").unwrap()));
    }

    #[test]
    fn test_decimal() {
        let src = "123.456789123456";
        let (num, errs) = parser().parse(src).into_output_errors();
        println!("{num:#?}");
        if !errs.is_empty() {
            print_report(errs, src);
            panic!("Parsing errors found");
        }

        assert_eq!(num.unwrap(), DatexExpression::Decimal(Decimal::from_string("123.456789123456")));
    }

    #[test]
    fn test_decimal_with_exponent() {
        let src = "1.23456789123456e2";
        let (num, errs) = parser().parse(src).into_output_errors();
        println!("{num:#?}");
        if !errs.is_empty() {
            print_report(errs, src);
            panic!("Parsing errors found");
        }

        assert_eq!(num.unwrap(), DatexExpression::Decimal(Decimal::from_string("123.456789123456")));
    }

    #[test]
    fn test_text_double_quotes() {
        let src = r#""Hello, world!""#;
        let (text, errs) = parser().parse(src).into_output_errors();
        println!("{text:#?}");
        if !errs.is_empty() {
            print_report(errs, src);
            panic!("Parsing errors found");
        }

        assert_eq!(text.unwrap(), DatexExpression::Text("Hello, world!".to_string()));
    }

    #[test]
    fn test_text_single_quotes() {
        let src = r#"'Hello, world!'"#;
        let (text, errs) = parser().parse(src).into_output_errors();
        println!("{text:#?}");
        if !errs.is_empty() {
            print_report(errs, src);
            panic!("Parsing errors found");
        }

        assert_eq!(text.unwrap(), DatexExpression::Text("Hello, world!".to_string()));
    }

    #[test]
    fn test_text_escape_sequences() {
        let src = r#""Hello, \"world\"! \n New line \t tab \uD83D\uDE00""#;
        let (text, errs) = parser().parse(src).into_output_errors();
        println!("{text:#?}");
        if !errs.is_empty() {
            print_report(errs, src);
            panic!("Parsing errors found");
        }

        assert_eq!(text.unwrap(), DatexExpression::Text("Hello, \"world\"! \n New line \t tab 😀".to_string()));
    }


    #[test]
    fn test_empty_array() {
        let src = "[]";
        let (arr, errs) = parser().parse(src).into_output_errors();
        println!("{arr:#?}");
        if !errs.is_empty() {
            print_report(errs, src);
            panic!("Parsing errors found");
        }

        assert_eq!(arr.unwrap(), DatexExpression::Array(vec![]));
    }

    #[test]
    fn test_array_with_values() {
        let src = "[1, 2, 3, 4.5, \"text\"]";
        let (arr, errs) = parser().parse(src).into_output_errors();
        println!("{arr:#?}");
        if !errs.is_empty() {
            print_report(errs, src);
            panic!("Parsing errors found");
        }

        assert_eq!(arr.unwrap(), DatexExpression::Array(vec![
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
        let (obj, errs) = parser().parse(src).into_output_errors();
        println!("{obj:#?}");
        if !errs.is_empty() {
            print_report(errs, src);
            panic!("Parsing errors found");
        }

        assert_eq!(obj.unwrap(), DatexExpression::Object(vec![]));
    }

    #[test]
    fn test_tuple() {
        let src = "1,2";
        let (tuple, errs) = parser().parse(src).into_output_errors();
        println!("{tuple:#?}");
        if !errs.is_empty() {
            print_report(errs, src);
        }
        assert_eq!(tuple.unwrap(), DatexExpression::Tuple(vec![
            TupleEntry::ValueOnly(DatexExpression::Integer(Integer::from(1))),
            TupleEntry::ValueOnly(DatexExpression::Integer(Integer::from(2))),
        ]));
    }

    #[test]
    fn test_single_value_tuple() {
        let src = "1,";
        let (tuple, errs) = parser().parse(src).into_output_errors();
        println!("{tuple:#?}");
        if !errs.is_empty() {
            print_report(errs, src);
            panic!("Parsing errors found");
        }
        assert_eq!(tuple.unwrap(), DatexExpression::Tuple(vec![
            TupleEntry::ValueOnly(DatexExpression::Integer(Integer::from(1))),
        ]));
    }

    #[test]
    fn test_scoped_atom() {
        let src = "(1)";
        let (atom, errs) = parser().parse(src).into_output_errors();
        println!("{atom:#?}");
        if !errs.is_empty() {
            print_report(errs, src);
            panic!("Parsing errors found");
        }
        assert_eq!(atom.unwrap(), DatexExpression::Integer(Integer::from(1)));
    }

    #[test]
    fn test_object_with_key_value_pairs() {
        let src = r#"{"key1": "value1", "key2": 42, "key3": true}"#;
        let (obj, errs) = parser().parse(src).into_output_errors();
        println!("{obj:#?}");
        if !errs.is_empty() {
            print_report(errs, src);
            panic!("Parsing errors found");
        }

        assert_eq!(obj.unwrap(), DatexExpression::Object(vec![
            (DatexExpression::Text("key1".to_string()), DatexExpression::Text("value1".to_string())),
            (DatexExpression::Text("key2".to_string()), DatexExpression::Integer(Integer::from(42))),
            (DatexExpression::Text("key3".to_string()), DatexExpression::Bool(true)),
        ]));
    }

}