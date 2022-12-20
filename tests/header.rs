use datex_core::global::dxb_block::{DXBHeader, RoutingInfo};
use datex_core::utils::buffers::{hex_to_buffer_advanced, buffer_to_hex};
use datex_core::parser::header::parse_dxb_header;
use datex_core::generator::header::generate_dxb_header;

// const CTX:&LoggerContext = &LoggerContext {log_redirect:None};

#[test]
pub fn parse_header() {
	// dxb -> header
    let dxb = hex_to_buffer_advanced("01 64 02 00 00 ff 01 03".to_string(), " ");
    let (header, body) = parse_dxb_header(&dxb);

	assert_eq!(header.version, 2);
	assert_eq!(header.signed, false);
	assert_eq!(header.encrypted, true);
	assert_eq!(header.routing.ttl, 0xff);
	assert_eq!(header.routing.priority, 1);

	println!("{:#?}", header);
}

#[test]
pub fn generate_header() {
	// dxb -> header -> dxb
    let header_dxb = hex_to_buffer_advanced("01 64 02 00 00 ff 01 03".to_string(), " ");
    let (header, body) = parse_dxb_header(&header_dxb);
	let gen_header_dxb = generate_dxb_header(&header);
	assert_eq!(buffer_to_hex(header_dxb), buffer_to_hex(gen_header_dxb));

	// header -> dxb -> header
	let header = DXBHeader {
		version: 1,
		size: 65535,
		signed: true,
		encrypted: true,
		routing: RoutingInfo {
			ttl: 14,
			priority: 40
		}
	};
	assert_eq!(parse_dxb_header(&generate_dxb_header(&header)).0, header);

}