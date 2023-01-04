use crate::{utils::{buffers::{read_u8, read_u16, read_slice, read_u32, read_u64}}, global::dxb_block::{DXBHeader, RoutingInfo, HeaderFlags, DXBBlockType}, datex_values::Endpoint};


// checks magic number
pub fn has_dxb_magic_number(dxb:&[u8]) -> bool {
	dxb[0] == 0x01 && dxb[1] == 0x64
}

pub fn parse_dxb_header<'a>(dxb:&'a [u8]) -> (DXBHeader, &'a [u8]) {
	// has magic number?
	if !has_dxb_magic_number(dxb) {
		panic!("Invalid DXB header format - missing magic number");
	}
	// header to short
	if dxb.len() < 28 {
		panic!("Invalid DXB header format - too short");
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

	// block header
	let scope_id = read_u32(dxb, index);
	let block_index = read_u16(dxb, index);
	let block_increment = read_u16(dxb, index);
	let block_type = DXBBlockType::try_from(read_u8(dxb, index)).expect("Invalid DXB block type");
	let flags = read_u8(dxb, index); // TODO: parse
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

		flags: HeaderFlags {allow_execute:true,end_of_scope:true,device_type:0},
		routing: RoutingInfo {ttl, sender, priority}
	};

	return (header, dxb.clone());
}


fn get_dxb_header_sender(dxb:&[u8], index: &mut usize) -> Option<Endpoint> {

	if read_u16(dxb, index) == std::u16::MAX {
		return None;
	}
	else {
		*index -= 2;
		return Some(Endpoint::new_from_binary(&read_slice(dxb, index, 21)))
	}
}