use std::fmt::Display;
use chumsky::error::Rich;
use pest::error::Error;
use crate::global::dxb_block::DXBBlock;
use crate::global::protocol_structures::block_header::BlockHeader;
use crate::global::protocol_structures::encrypted_header::EncryptedHeader;
use crate::global::protocol_structures::routing_header;
use crate::global::protocol_structures::routing_header::RoutingHeader;

pub mod bytecode;
mod parser;

use crate::datex_values::core_values::endpoint::Endpoint;
use crate::compiler::bytecode::compile_script;
use crate::compiler::parser::DatexExpression;

#[derive(Debug)]
pub enum CompilerError<'a> {
    UnexpectedTerm(DatexExpression),
    SyntaxError(Vec<Rich<'a, char>>),
    SerializationError(binrw::Error),
    BigDecimalOutOfBoundsError,
    IntegerOutOfBoundsError,
    InvalidPlaceholderCount,
    NonStaticValue
}

impl<'a> From<Vec<Rich<'a, char>>> for CompilerError<'a> {
    fn from(error: Vec<Rich<'a, char>>) -> Self {
        CompilerError::SyntaxError(error)
    }
}

impl<'a> Display for CompilerError<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompilerError::UnexpectedTerm(rule) => {
                write!(f, "Unexpected term: {rule:?}")
            }
            CompilerError::SyntaxError(error) => {
                write!(f, "Syntax error") // TODO
            }
            CompilerError::SerializationError(error) => {
                write!(f, "Serialization error: {error}")
            }
            CompilerError::BigDecimalOutOfBoundsError => {
                write!(f, "BigDecimal out of bounds error")
            }
            CompilerError::IntegerOutOfBoundsError => {
                write!(f, "Integer out of bounds error")
            }
            CompilerError::InvalidPlaceholderCount => {
                write!(f, "Invalid placeholder count")
            }
            CompilerError::NonStaticValue => {
                write!(f, "Encountered non-static value")
            }
        }
    }
}

pub fn compile_block(datex_script: &str) -> Result<Vec<u8>, CompilerError> {
    let body = compile_script(datex_script)?;

    let routing_header = RoutingHeader {
        version: 2,
        flags: routing_header::Flags::new(),
        block_size_u16: Some(0),
        block_size_u32: None,
        sender: Endpoint::LOCAL,
        receivers: routing_header::Receivers {
            flags: routing_header::ReceiverFlags::new()
                .with_has_endpoints(false)
                .with_has_pointer_id(false)
                .with_has_endpoint_keys(false),
            pointer_id: None,
            endpoints: None,
            endpoints_with_keys: None,
        },
        ..RoutingHeader::default()
    };

    let block_header = BlockHeader::default();
    let encrypted_header = EncryptedHeader::default();

    let block =
        DXBBlock::new(routing_header, block_header, encrypted_header, body);

    let bytes = block
        .to_bytes()
        .map_err(CompilerError::SerializationError)?;
    Ok(bytes)
}
