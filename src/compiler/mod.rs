use crate::global::dxb_block::DXBBlock;
use crate::global::protocol_structures::block_header::BlockHeader;
use crate::global::protocol_structures::encrypted_header::EncryptedHeader;
use crate::global::protocol_structures::routing_header;
use crate::global::protocol_structures::routing_header::RoutingHeader;
use strum::Display;


mod operations;
pub mod parser;
pub mod bytecode;

use crate::datex_values::core_values::endpoint::Endpoint;
use crate::compiler::bytecode::compile_script;

#[derive(Debug, Display)]
pub enum CompilationError {
    InvalidRule(String),
    SerializationError(binrw::Error),
}

pub fn compile_block(datex_script: &str) -> Result<Vec<u8>, CompilationError> {
    let body = compile_script(datex_script)
        .map_err(|e| CompilationError::InvalidRule(e.to_string()))?;

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
        .map_err(CompilationError::SerializationError)?;
    Ok(bytes)
}
