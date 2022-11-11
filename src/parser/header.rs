use crate::Logger;

pub fn parse_dxb_header(dxb_header:&[u8]) -> &[u8] {
	let logger:Logger = Logger::new("DATEX WASM Parser");

	logger.info(&format!("Parsing DXB header"));

	dxb_header
}