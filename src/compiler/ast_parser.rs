use crate::compiler::lexer::{DecimalLiteral, IntegerLiteral, Token};
use crate::global::binary_codes::InstructionCode;
use crate::global::protocol_structures::instructions::Instruction;
use crate::values::core_values::array::Array;
use crate::values::core_values::decimal::decimal::Decimal;
use crate::values::core_values::decimal::typed_decimal::TypedDecimal;
use crate::values::core_values::integer::integer::Integer;
use crate::values::core_values::integer::typed_integer::TypedInteger;
use crate::values::core_values::object::Object;
use crate::values::value::Value;
use crate::values::value_container::ValueContainer;
use crate::{
    compiler::ast_parser::extra::Err, values::core_values::endpoint::Endpoint,
};
use chumsky::prelude::*;
use logos::Logos;
use std::str::FromStr;
use std::{collections::HashMap, ops::Range};

#[derive(Clone, Debug, PartialEq)]
pub enum TupleEntry {
    KeyValue(DatexExpression, DatexExpression),
    Value(DatexExpression),
}

#[derive(Clone, Debug, PartialEq, Copy)]
pub enum BinaryOperator {
    Add,          // +
    Subtract,     // -
    Multiply,     // *
    Divide,       // /
    Modulo,       // %
    Power,        // ^
    And,          // &&
    Or,           // ||
    CompositeAnd, // TODO
    CompositeOr,  // TODO
}

#[derive(Clone, Debug, PartialEq, Copy)]
pub enum ComparisonOperator {
    Is,                 // is
    StructuralEqual,    // ==
    NotStructuralEqual, // !=
    Equal,              // ===
    NotEqual,           // !==
    LessThan,           // <
    GreaterThan,        // >
    LessThanOrEqual,    // <=
    GreaterThanOrEqual, // >=
}

#[derive(Clone, Debug, PartialEq, Copy)]
pub enum AssignmentOperator {
    Assign,          // =
    AddAssign,       // +=
    SubstractAssign, // -=
    MultiplyAssign,  // *=
    DivideAssign,    // /=
}

impl From<&ComparisonOperator> for InstructionCode {
    fn from(op: &ComparisonOperator) -> Self {
        match op {
            ComparisonOperator::StructuralEqual => {
                InstructionCode::STRUCTURAL_EQUAL
            }
            ComparisonOperator::NotStructuralEqual => {
                InstructionCode::NOT_STRUCTURAL_EQUAL
            }
            ComparisonOperator::Equal => InstructionCode::EQUAL,
            ComparisonOperator::NotEqual => InstructionCode::NOT_EQUAL,
            ComparisonOperator::Is => InstructionCode::IS,
            operator => todo!(
                "Comparison operator {:?} not implemented for InstructionCode",
                operator
            ),
        }
    }
}

impl From<ComparisonOperator> for InstructionCode {
    fn from(op: ComparisonOperator) -> Self {
        InstructionCode::from(&op)
    }
}

impl From<&AssignmentOperator> for InstructionCode {
    fn from(op: &AssignmentOperator) -> Self {
        match op {
            AssignmentOperator::Assign => InstructionCode::ASSIGN,
            AssignmentOperator::AddAssign => InstructionCode::ADD_ASSIGN,
            AssignmentOperator::SubstractAssign => {
                InstructionCode::SUBTRACT_ASSIGN
            }
            AssignmentOperator::MultiplyAssign => {
                InstructionCode::MULTIPLY_ASSIGN
            }
            AssignmentOperator::DivideAssign => InstructionCode::DIVIDE_ASSIGN,
            operator => todo!(
                "Assignment operator {:?} not implemented for InstructionCode",
                operator
            ),
        }
    }
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
            operator => todo!(
                "Binary operator {:?} not implemented for InstructionCode",
                operator
            ),
        }
    }
}

impl From<BinaryOperator> for InstructionCode {
    fn from(op: BinaryOperator) -> Self {
        InstructionCode::from(&op)
    }
}

impl From<&InstructionCode> for BinaryOperator {
    fn from(code: &InstructionCode) -> Self {
        match code {
            InstructionCode::ADD => BinaryOperator::Add,
            InstructionCode::SUBTRACT => BinaryOperator::Subtract,
            InstructionCode::MULTIPLY => BinaryOperator::Multiply,
            InstructionCode::DIVIDE => BinaryOperator::Divide,
            InstructionCode::MODULO => BinaryOperator::Modulo,
            InstructionCode::POWER => BinaryOperator::Power,
            InstructionCode::AND => BinaryOperator::And,
            InstructionCode::OR => BinaryOperator::Or,
            _ => todo!("#154 Binary operator for {:?} not implemented", code),
        }
    }
}

impl From<InstructionCode> for BinaryOperator {
    fn from(code: InstructionCode) -> Self {
        BinaryOperator::from(&code)
    }
}

impl From<&Instruction> for BinaryOperator {
    fn from(instruction: &Instruction) -> Self {
        match instruction {
            Instruction::Add => BinaryOperator::Add,
            Instruction::Subtract => BinaryOperator::Subtract,
            Instruction::Multiply => BinaryOperator::Multiply,
            Instruction::Divide => BinaryOperator::Divide,
            _ => {
                todo!(
                    "#155 Binary operator for instruction {:?} not implemented",
                    instruction
                );
            }
        }
    }
}

impl From<Instruction> for BinaryOperator {
    fn from(instruction: Instruction) -> Self {
        BinaryOperator::from(&instruction)
    }
}

impl From<&Instruction> for ComparisonOperator {
    fn from(instruction: &Instruction) -> Self {
        match instruction {
            Instruction::StructuralEqual => ComparisonOperator::StructuralEqual,
            Instruction::Equal => ComparisonOperator::Equal,
            Instruction::NotStructuralEqual => {
                ComparisonOperator::NotStructuralEqual
            }
            Instruction::NotEqual => ComparisonOperator::NotEqual,
            Instruction::Is => ComparisonOperator::Is,
            _ => {
                todo!(
                    "Comparison operator for instruction {:?} not implemented",
                    instruction
                );
            }
        }
    }
}

impl From<Instruction> for ComparisonOperator {
    fn from(instruction: Instruction) -> Self {
        ComparisonOperator::from(&instruction)
    }
}

#[derive(Clone, Debug, PartialEq, Copy)]
pub enum UnaryOperator {
    Negate,
    CreateRef,
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

// TODO TBD can we deprecate this?
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum VariableType {
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
    VariableDeclaration(
        Option<VariableId>,
        VariableType,
        BindingMutability,
        ReferenceMutability,
        String,
        Box<DatexExpression>,
    ),

    /// Reference, e.g. &x
    Ref(Box<DatexExpression>),
    /// Mutable reference, e.g. &mut x
    RefMut(Box<DatexExpression>),

    /// Variable assignment, e.g. x = 1. VariableId is always set to 0 by the ast parser.
    // VariableAssignment(Option<VariableId>, String, Box<DatexExpression>),

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
    ApplyChain(Box<DatexExpression>, Vec<Apply>),
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
        // TODO #156 remove all other backslashes before any other character
        .to_string();
    // Decode unicode escapes, e.g. \u1234 or \uD800\uDC00
    decode_json_unicode_escapes(&escaped)
}

fn binary_op(
    op: BinaryOperator,
) -> impl Fn(Box<DatexExpression>, Box<DatexExpression>) -> DatexExpression + Clone
{
    move |lhs, rhs| DatexExpression::BinaryOperation(op, lhs, rhs)
}

fn comparison_op(
    op: ComparisonOperator,
) -> impl Fn(Box<DatexExpression>, Box<DatexExpression>) -> DatexExpression + Clone
{
    move |lhs, rhs| DatexExpression::ComparisonOperation(op, lhs, rhs)
}

fn assignment_op(
    op: AssignmentOperator,
) -> impl Fn(String, Box<DatexExpression>) -> DatexExpression + Clone {
    move |lhs, rhs| DatexExpression::AssignmentOperation(op, None, lhs, rhs)
}

pub struct DatexParseResult {
    pub expression: DatexExpression,
    pub is_static_value: bool,
}

pub fn create_parser<'a, I>()
-> impl Parser<'a, TokenInput<'a>, DatexExpression, Err<Cheap>>
// where
//     I: SliceInput<'a, Token = Token, Span = SimpleSpan>,
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
        Token::DecimalIntegerLiteral(IntegerLiteral { value, variant }) => {
            match variant {
                Some(var) => TypedInteger::from_string_with_variant(&value, var)
                    .map(DatexExpression::TypedInteger)
                    .unwrap_or(DatexExpression::Invalid),
                None => Integer::from_string(&value)
                    .map(DatexExpression::Integer)
                    .unwrap_or(DatexExpression::Invalid),
            }
        },
        Token::BinaryIntegerLiteral(IntegerLiteral { value, variant }) => {
            match variant {
                Some(var) => TypedInteger::from_string_radix_with_variant(&value[2..], 2, var)
                    .map(DatexExpression::TypedInteger)
                    .unwrap_or(DatexExpression::Invalid),
                None => Integer::from_string_radix(&value[2..], 2)
                    .map(DatexExpression::Integer)
                    .unwrap_or(DatexExpression::Invalid),
            }
        },
        Token::HexadecimalIntegerLiteral(IntegerLiteral { value, variant }) => {
            match variant {
                Some(var) => TypedInteger::from_string_radix_with_variant(&value[2..], 16, var)
                    .map(DatexExpression::TypedInteger)
                    .unwrap_or(DatexExpression::Invalid),
                None => Integer::from_string_radix(&value[2..], 16)
                    .map(DatexExpression::Integer)
                    .unwrap_or(DatexExpression::Invalid),
            }
        },
        Token::OctalIntegerLiteral(IntegerLiteral { value, variant }) => {
            match variant {
                Some(var) => TypedInteger::from_string_radix_with_variant(&value[2..], 8, var)
                    .map(DatexExpression::TypedInteger)
                    .unwrap_or(DatexExpression::Invalid),
                None => Integer::from_string_radix(&value[2..], 8)
                    .map(DatexExpression::Integer)
                    .unwrap_or(DatexExpression::Invalid),
            }
        },
    };
    let decimal = select! {
        Token::DecimalLiteral(DecimalLiteral { value, variant }) => {
            match variant {
                Some(var) => TypedDecimal::from_string_with_variant(&value, var)
                    .map(DatexExpression::TypedDecimal)
                    .unwrap_or(DatexExpression::Invalid),
                None => DatexExpression::Decimal(Decimal::from_string(&value))
            }
        },
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
    let endpoint = select! {
        Token::Endpoint(s) =>
            match Endpoint::from_str(s.as_str()) {
                Err(_) => DatexExpression::Invalid,
                Ok(endpoint) => DatexExpression::Endpoint(endpoint)
        }
    };
    let literal = select! {
        Token::TrueKW => DatexExpression::Boolean(true),
        Token::FalseKW => DatexExpression::Boolean(false),
        Token::NullKW => DatexExpression::Null,
        Token::Identifier(s) => DatexExpression::Variable(None, s),
        Token::NamedSlot(s) => DatexExpression::Slot(Slot::Named(s[1..].to_string())),
        Token::Slot(s) => DatexExpression::Slot(Slot::Addressed(
            // replace first char (#) and convert to u32
            s[1..].parse::<u32>().unwrap()
        )),
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
        endpoint,
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
        endpoint,
        wrapped_expression.clone(),
    ))
    .boxed();

    let unary = recursive(|unary| {
        // & or &mut prefix
        just(Token::Ampersand)
            .ignore_then(
                just(Token::MutKW).or_not().padded_by(whitespace.clone()),
            )
            .then(unary.clone())
            .map(|(mut_kw, expr)| {
                if mut_kw.is_some() {
                    DatexExpression::RefMut(Box::new(expr))
                } else {
                    DatexExpression::Ref(Box::new(expr))
                }
            })
            // could also add unary minus, not, etc. here later
            .or(atom.clone())
    });

    // operations on atoms
    let op = |c| {
        just(Token::Whitespace)
            .repeated()
            .at_least(1)
            .ignore_then(just(c))
            .then_ignore(just(Token::Whitespace).repeated().at_least(1))
    };

    // apply chain: two expressions following each other directly, optionally separated with "." (property access)
    let apply_or_property_access = unary
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

    // equality
    let equality = sum.clone().foldl(
        choice((
            op(Token::StructuralEqual) //  ==
                .to(comparison_op(ComparisonOperator::StructuralEqual)),
            op(Token::Equal) //  ===
                .to(comparison_op(ComparisonOperator::Equal)),
            op(Token::NotStructuralEqual) //  !=
                .to(comparison_op(ComparisonOperator::NotStructuralEqual)),
            op(Token::NotEqual) //  !==
                .to(comparison_op(ComparisonOperator::NotEqual)),
            op(Token::Is) //  is
                .to(comparison_op(ComparisonOperator::Is)),
            // op(Token::LessThan) //  <
            //     .to(binary_op(BinaryOperator::LessThan)),
            // op(Token::GreaterThan) //  >
            //     .to(binary_op(BinaryOperator::GreaterThan)),
            // op(Token::LessThanOrEqual) //  <=
            //     .to(binary_op(BinaryOperator::LessThanOrEqual)),
            // op(Token::GreaterThanOrEqual) //  >=
            //     .to(binary_op(BinaryOperator::GreaterThanOrEqual)),
        ))
        .then(sum)
        .repeated(), // allows chaining like a == b == c
        |lhs, (op, rhs)| op(Box::new(lhs), Box::new(rhs)),
    );

    let assignment_op = select! {
        Token::Assign      => AssignmentOperator::Assign,
        Token::AddAssign   => AssignmentOperator::AddAssign,
        Token::SubAssign   => AssignmentOperator::SubstractAssign,
        Token::MulAssign   => AssignmentOperator::MultiplyAssign,
        Token::DivAssign   => AssignmentOperator::DivideAssign,
    }
    .padded_by(whitespace.clone());

    // variable declarations or assignments
    let variable_assignment = just(Token::ConstKW)
        .or(just(Token::VarKW))
        .or_not()
        .padded_by(whitespace.clone())
        .then(select! {
            Token::Identifier(s) => s
        })
        .then(assignment_op)
        .then(equality.clone())
        .map(|(((var_type, var_name), op), expr)| {
            if let Some(var_type) = var_type {
                let (mutability, expr) = match expr {
                    DatexExpression::RefMut(expr) => {
                        (ReferenceMutability::Mutable, expr)
                    }

                    DatexExpression::Ref(expr) => {
                        (ReferenceMutability::Immutable, expr)
                    }

                    expr => (ReferenceMutability::None, Box::new(expr)),
                };
                if op != AssignmentOperator::Assign {
                    return DatexExpression::Invalid;
                }
                DatexExpression::VariableDeclaration(
                    None,
                    if var_type == Token::ConstKW {
                        VariableType::Const
                    } else {
                        VariableType::Var
                    },
                    if var_type == Token::ConstKW {
                        BindingMutability::Immutable
                    } else {
                        BindingMutability::Mutable
                    },
                    mutability,
                    var_name.to_string(),
                    expr,
                )
            } else {
                DatexExpression::AssignmentOperation(
                    op,
                    None,
                    var_name.to_string(),
                    Box::new(expr),
                )
            }
        });

    expression_without_tuple
        .define(choice((variable_assignment, equality.clone())));

    // expression :: expression
    let remote_execution = expression_without_tuple
        .clone()
        .then_ignore(just(Token::DoubleColon).padded_by(whitespace.clone()))
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
    UnexpectedToken(Range<usize>),
    InvalidToken(Range<usize>),
}

impl From<Range<usize>> for ParserError {
    fn from(range: Range<usize>) -> Self {
        ParserError::InvalidToken(range)
    }
}

pub fn parse(mut src: &str) -> Result<DatexExpression, Vec<ParserError>> {
    // strip shebang at beginning of the source code
    if src.starts_with("#!") {
        let end_of_line = src.find('\n').unwrap_or(src.len());
        src = &src[end_of_line + 1..];
    }

    let tokens = Token::lexer(src);
    let tokens: Vec<Token> = tokens
        .into_iter()
        .collect::<Result<Vec<Token>, Range<usize>>>()
        .map_err(|e| vec![ParserError::InvalidToken(e)])?;

    let parser = create_parser::<'_, TokenInput>();

    parser.parse(&tokens).into_result().map_err(|err| {
        err.into_iter()
            .map(|e| ParserError::UnexpectedToken(e.span().into_range()))
            .collect()
    })
}

// TODO #157: implement correctly - have fun with lifetimes :()
// mainly relevant for IDE language support
// pub fn parse_with_context(src: &str, parser) -> (DatexExpression, Vec<ParserError>) {
//     let lexer = Token::lexer(src);
//     let tokens = lexer.spanned().map(|(tok, span)| match tok {
//         Ok(tok) => (tok, span.into()),
//         Err(_) => (Token::Error, span.into()),
//     });
//     let tokens = Stream::from_iter(tokens)
//         .map((0..src.len()).into(), |(t, s): (_, _)| (t, s));

//     let result = parser.parse(tokens).into_result().map_err(|err| {
//         err.into_iter()
//             .map(|e| ParserError::UnexpectedToken(e.span().into_range()))
//             .collect()
//     });
//     result
// }

#[cfg(test)]
mod tests {

    use super::*;

    use std::assert_matches::assert_matches;

    fn print_report(errs: Vec<ParserError>, src: &str) {
        // FIXME #158
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
                    Box::new(DatexExpression::Variable(None, "x".to_string())),
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
        assert_eq!(expr, DatexExpression::Variable(None, "myVar".to_string()));
    }

    #[test]
    fn test_variable_expression_with_operations() {
        let src = "myVar + 1";
        let expr = parse_unwrap(src);
        assert_eq!(
            expr,
            DatexExpression::BinaryOperation(
                BinaryOperator::Add,
                Box::new(DatexExpression::Variable(None, "myVar".to_string())),
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
                Box::new(DatexExpression::Variable(None, "myFunc".to_string())),
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
                Box::new(DatexExpression::Variable(None, "myFunc".to_string())),
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
                Box::new(DatexExpression::Variable(None, "myFunc".to_string())),
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
                Box::new(DatexExpression::Variable(None, "print".to_string())),
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
                Box::new(DatexExpression::Variable(None, "myObj".to_string())),
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
                Box::new(DatexExpression::Variable(None, "myObj".to_string())),
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
                Box::new(DatexExpression::Variable(None, "myObj".to_string())),
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
                                None,
                                "x".to_string()
                            ),
                            is_terminated: true,
                        },
                        Statement {
                            expression: DatexExpression::Variable(
                                None,
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
                Box::new(DatexExpression::Variable(None, "myObj".to_string())),
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
                Box::new(DatexExpression::Variable(None, "myFunc".to_string())),
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
                        Box::new(DatexExpression::Variable(
                            None,
                            "x".to_string()
                        )),
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
    fn variable_declaration_statement() {
        let src = "const x = 42;";
        let expr = parse_unwrap(src);
        assert_eq!(
            expr,
            DatexExpression::Statements(vec![Statement {
                expression: DatexExpression::VariableDeclaration(
                    None,
                    VariableType::Const,
                    BindingMutability::Immutable,
                    ReferenceMutability::None,
                    "x".to_string(),
                    Box::new(DatexExpression::Integer(Integer::from(42))),
                ),
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
            DatexExpression::VariableDeclaration(
                None,
                VariableType::Var,
                BindingMutability::Mutable,
                ReferenceMutability::None,
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
                Box::new(DatexExpression::Variable(None, "myFunc".to_string())),
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
                    expression: DatexExpression::VariableDeclaration(
                        None,
                        VariableType::Var,
                        BindingMutability::Mutable,
                        ReferenceMutability::None,
                        "x".to_string(),
                        Box::new(DatexExpression::Integer(Integer::from(42))),
                    ),
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
                Box::new(DatexExpression::Variable(None, "a".to_string())),
                Box::new(DatexExpression::Variable(None, "b".to_string()))
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
                Box::new(DatexExpression::Variable(None, "a".to_string())),
                Box::new(DatexExpression::Variable(None, "b".to_string()))
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
                Box::new(DatexExpression::Variable(None, "a".to_string())),
                Box::new(DatexExpression::BinaryOperation(
                    BinaryOperator::Add,
                    Box::new(DatexExpression::Variable(None, "b".to_string())),
                    Box::new(DatexExpression::BinaryOperation(
                        BinaryOperator::Multiply,
                        Box::new(DatexExpression::Variable(
                            None,
                            "c".to_string()
                        )),
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
                        Box::new(DatexExpression::Variable(
                            None,
                            "a".to_string()
                        )),
                        Box::new(DatexExpression::Variable(
                            None,
                            "b".to_string()
                        ))
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
                Box::new(DatexExpression::Variable(None, "a".to_string())),
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
            DatexExpression::VariableDeclaration(
                None,
                VariableType::Const,
                BindingMutability::Immutable,
                ReferenceMutability::Mutable,
                "x".to_string(),
                Box::new(DatexExpression::Array(vec![
                    DatexExpression::Integer(Integer::from(1)),
                    DatexExpression::Integer(Integer::from(2)),
                    DatexExpression::Integer(Integer::from(3)),
                ])),
            )
        );
    }

    #[test]
    fn variable_declaration_ref() {
        let src = "const x = &[1, 2, 3]";
        let expr = parse_unwrap(src);
        assert_eq!(
            expr,
            DatexExpression::VariableDeclaration(
                None,
                VariableType::Const,
                BindingMutability::Immutable,
                ReferenceMutability::Immutable,
                "x".to_string(),
                Box::new(DatexExpression::Array(vec![
                    DatexExpression::Integer(Integer::from(1)),
                    DatexExpression::Integer(Integer::from(2)),
                    DatexExpression::Integer(Integer::from(3)),
                ])),
            )
        );
    }
    #[test]
    fn variable_declaration() {
        let src = "const x = 1";
        let expr = parse_unwrap(src);
        assert_eq!(
            expr,
            DatexExpression::VariableDeclaration(
                None,
                VariableType::Const,
                BindingMutability::Immutable,
                ReferenceMutability::None,
                "x".to_string(),
                Box::new(DatexExpression::Integer(Integer::from(1))),
            )
        );
    }
}
