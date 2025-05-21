use pest::iterators::Pair;
use crate::compiler::parser::Rule;
use crate::global::binary_codes::BinaryCode;

#[derive(Debug, Clone, PartialEq)]
pub enum OperationMode {
    Add,
    Subtract,
    Multiply,
    Divide,
}
impl From<OperationMode> for BinaryCode {
    fn from(mode: OperationMode) -> Self {
        match mode {
            OperationMode::Add => BinaryCode::ADD,
            OperationMode::Subtract => BinaryCode::SUBTRACT,
            OperationMode::Multiply => BinaryCode::MULTIPLY,
            OperationMode::Divide => BinaryCode::DIVIDE,
        }
    }
}

impl OperationMode {
    pub fn from_string_additive(operator: &str) -> Self {
        match operator {
            "+" => OperationMode::Add,
            "-" => OperationMode::Subtract,
            _ => unreachable!("Expected + or -, but found {}", operator),
        }
    }
    pub fn from_string_multiplicative(operator: &str) -> Self {
        match operator {
            "*" => OperationMode::Multiply,
            "/" => OperationMode::Divide,
            _ => unreachable!("Expected * or /, but found {}", operator),
        }
    }
}


pub fn parse_operator(
    pair: Pair<'_, Rule>,
) -> OperationMode {
    // assert_eq!(pair.as_rule(), Rule::operator, "Expected Rule::operator");
    let operator = pair.as_str();
    match pair.as_rule() {
        Rule::additive_operator => {
            OperationMode::from_string_additive(operator)
        }
        Rule::multiplicative_operator => {
            OperationMode::from_string_multiplicative(operator)
        }
        _ => unreachable!("Expected +, -, *, /, but found {}", operator),
    }
}