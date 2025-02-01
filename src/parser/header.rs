use crate::{
    datex_values::Endpoint,
    global::dxb_header::{DXBBlockType, DXBHeader, HeaderFlags, RoutingInfo},
    utils::buffers::{read_slice, read_u16, read_u32, read_u64, read_u8},
};

// checks magic number
pub fn has_dxb_magic_number(dxb: &[u8]) -> bool {
    dxb.len() >= 2 && dxb[0] == 0x01 && dxb[1] == 0x64
}


#[derive(Debug)]
pub enum HeaderParsingError {
    InvalidMagicNumber,
    InsufficientLength,
}


pub fn extract_dxb_block_length(dxb: &[u8]) -> Result<u16, HeaderParsingError> {
    if !has_dxb_magic_number(dxb) {return Err(HeaderParsingError::InvalidMagicNumber)}
    if dxb.len() < 6 {return Err(HeaderParsingError::InsufficientLength)}
    return Ok(read_u16(dxb, &mut 4));
}

pub fn parse_dxb_header<'a>(dxb: &'a [u8]) -> Result<DXBHeader, HeaderParsingError> {
    // has magic number?
    if !has_dxb_magic_number(dxb) {
        return Err(HeaderParsingError::InvalidMagicNumber);
    }
    // header to short
    if dxb.len() < 28 {
        return Err(HeaderParsingError::InsufficientLength);
    }

    let index = &mut 2;

    // pre header
    let version = read_u8(dxb, index);
    let size = read_u16(dxb, index);
    let ttl = read_u8(dxb, index);
    let priority = read_u8(dxb, index);
    let signed_encrypted = read_u8(dxb, index);
    let signed = signed_encrypted == 1 || signed_encrypted == 2; // is signed?
    let encrypted = signed_encrypted == 2 || signed_encrypted == 3; // is encrypted?
    let sender = get_dxb_header_sender(dxb, index);
    let _receivers = get_dxb_header_receivers(dxb, index);

    // block header
    let scope_id = read_u32(dxb, index);
    let block_index = read_u16(dxb, index);
    let block_increment = read_u16(dxb, index);
    let block_type = DXBBlockType::try_from(read_u8(dxb, index)).expect("Invalid DXB block type");
    let _flags = read_u8(dxb, index); // TODO: parse
    let timestamp = read_u64(dxb, index);

    let header = DXBHeader {
        version,
        size,
        signed,
        encrypted,

        scope_id,
        block_index,
        block_increment,
        block_type,
        timestamp,

        flags: HeaderFlags {
            allow_execute: true,
            end_of_scope: true,
            device_type: 0,
        },
        routing: RoutingInfo {
            ttl,
            sender,
            priority,
        },
        body_start_offset: *index,
    };

    return Ok(header);
}

fn get_dxb_header_sender(dxb: &[u8], index: &mut usize) -> Option<Endpoint> {
    if read_u8(dxb, index) == std::u8::MAX {
        return None;
    } else {
        *index -= 1;
        return Some(Endpoint::new_from_binary(&read_slice(dxb, index, 21)));
    }
}

fn get_dxb_header_receivers(dxb: &[u8], index: &mut usize) -> Option<Endpoint> {
    if read_u16(dxb, index) == std::u16::MAX {
        return None;
    } else {
        // TODO:
        return None;
    }
}
