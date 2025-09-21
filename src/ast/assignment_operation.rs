use std::fmt::Display;

use crate::ast::DatexParserTrait;
use crate::ast::lexer::Token;
use crate::ast::utils::whitespace;
use crate::global::binary_codes::InstructionCode;
use chumsky::prelude::*;

#[derive(Clone, Debug, PartialEq, Copy)]
pub enum AssignmentOperator {
    Assign,           // =
    AddAssign,        // +=
    SubstractAssign,  // -=
    MultiplyAssign,   // *=
    DivideAssign,     // /=
    ModuloAssign,     // %=
    PowerAssign,      // ^=
    BitwiseAndAssign, // &=
    BitwiseOrAssign,  // |=
}
impl Display for AssignmentOperator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                AssignmentOperator::Assign => "=",
                AssignmentOperator::AddAssign => "+=",
                AssignmentOperator::SubstractAssign => "-=",
                AssignmentOperator::MultiplyAssign => "*=",
                AssignmentOperator::DivideAssign => "/=",
                AssignmentOperator::ModuloAssign => "%=",
                AssignmentOperator::PowerAssign => "^=",
                AssignmentOperator::BitwiseAndAssign => "&=",
                AssignmentOperator::BitwiseOrAssign => "|=",
            }
        )
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

pub fn assignment_operation<'a>()
-> impl DatexParserTrait<'a, AssignmentOperator> {
    select! {
        Token::Assign      => AssignmentOperator::Assign,
        Token::AddAssign   => AssignmentOperator::AddAssign,
        Token::SubAssign   => AssignmentOperator::SubstractAssign,
        Token::MulAssign   => AssignmentOperator::MultiplyAssign,
        Token::DivAssign   => AssignmentOperator::DivideAssign,
        Token::ModAssign   => AssignmentOperator::ModuloAssign,
    }
    .padded_by(whitespace())
}
