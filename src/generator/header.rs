
use crate::{global::dxb_block::DXBHeader, utils::buffers::{write_u8, append_u8, append_u16}};


pub fn generate_dxb_header<'a>(header:&DXBHeader) -> Vec<u8> {

	let dxb = &mut Vec::<u8>::with_capacity(200);

	let index = &mut 0;

	// magic number
	append_u8(dxb, 0x01);
	append_u8(dxb, 0x64);

	// version
	append_u8(dxb, header.version);

	// size
	append_u16(dxb, header.size);
	// routing
	append_u8(dxb, header.routing.ttl);
	append_u8(dxb, header.routing.priority);

	// signed/encrypted
	let signed_encrypted = if header.signed && header.encrypted {2} else if header.signed {1} else if header.encrypted {3} else {0};
	append_u8(dxb, signed_encrypted);

	return dxb.to_vec()
}