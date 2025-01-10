use crate::{
    global::dxb_block::DXBHeader,
    utils::buffers::{append_u16, append_u32, append_u64, append_u8},
};

pub fn append_dxb_header<'a>(header: &DXBHeader, _dxb: &[u8]) -> Vec<u8> {
    let pre_header = &mut generate_pre_header(header);
    let block_header = generate_block_header(header);

    pre_header.extend_from_slice(&block_header);

    return pre_header.to_vec();
}

fn generate_block_header(header: &DXBHeader) -> Vec<u8> {
    let block_header = &mut Vec::<u8>::with_capacity(200);

    // sid
    append_u32(block_header, header.scope_id);
    // block index
    append_u16(block_header, header.block_index);
    // block increment
    append_u16(block_header, header.block_increment);

    // type
    append_u8(block_header, header.block_type as u8);

    // TODO: flags
    append_u8(block_header, 0);

    // timestamp
    append_u64(block_header, header.timestamp);

    return block_header.to_vec();
}

fn generate_pre_header(header: &DXBHeader) -> Vec<u8> {
    let pre_header = &mut Vec::<u8>::with_capacity(200);

    let _index = &mut 0;

    // magic number
    append_u8(pre_header, 0x01);
    append_u8(pre_header, 0x64);

    // version
    append_u8(pre_header, header.version);

    // size
    append_u16(pre_header, header.size);
    // routing
    append_u8(pre_header, header.routing.ttl);
    append_u8(pre_header, header.routing.priority);

    // signed/encrypted
    let signed_encrypted = if header.signed && header.encrypted {
        2
    } else if header.signed {
        1
    } else if header.encrypted {
        3
    } else {
        0
    };
    append_u8(pre_header, signed_encrypted);

    // sender
    if header.routing.sender.is_some() {
        pre_header.extend_from_slice(&header.routing.sender.as_ref().unwrap().get_binary());
    }
    // no sender - anonymous
    else {
        append_u8(pre_header, std::u8::MAX); // 0xff
    }

    // TODO:
    // no receiver - flood
    // else {
    // 	append_u16(dxb, std::u16::MAX); // 0xffff
    // }
    append_u16(pre_header, std::u16::MAX); // 0xffff

    return pre_header.to_vec();
}
