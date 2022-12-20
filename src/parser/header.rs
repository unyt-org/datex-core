use crate::{utils::{logger::{LoggerContext, Logger}, buffers::{read_u8, read_u16}}, global::dxb_block::{DXBHeader, RoutingInfo}};



pub fn parse_dxb_header<'a>(dxb:&'a [u8]) -> (DXBHeader, &'a [u8]) {
	// has magic number?
	if dxb[0] != 0x01 || dxb[1] != 0x64 {
		panic!("Invalid DXB header format - missing magic number");
	}
	// header to short
	if dxb.len() < 8 {
		panic!("Invalid DXB header format - too short");
	}

	let index = &mut 2;

	let version = read_u8(dxb, index);
	let size = read_u16(dxb, index);
	let ttl = read_u8(dxb, index);
	let priority = read_u8(dxb, index);
	let signed_encrypted = read_u8(dxb, index);
	let signed = signed_encrypted == 1 || signed_encrypted == 2; // is signed?
	let encrypted = signed_encrypted == 2 || signed_encrypted == 3; // is encrypted?


	let header = DXBHeader {
		version,
		size,
		signed,
		encrypted,
		routing: RoutingInfo {ttl, priority}
	};

	return (header, dxb.clone());
}