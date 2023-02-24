use datex_core::datex_values::Endpoint;
use datex_core::global::dxb_block::{DXBHeader, RoutingInfo, DXBBlockType, HeaderFlags};
use datex_core::runtime::Runtime;
use datex_core::utils::buffers::{hex_to_buffer_advanced, buffer_to_hex};
use datex_core::parser::header::parse_dxb_header;
use datex_core::generator::header::append_dxb_header;

// const CTX:&LoggerContext = &LoggerContext {log_redirect:None};

/**
 * test if dxb header is correctly parsed into a DXBHeader struct
 */
#[test]
pub fn parse_header() {
	// dxb -> header
    let dxb = hex_to_buffer_advanced("01 64 02 00 00 ff 01 00 ff ff ff 03 00 00 00 04 00 05 00 00 01 09 00 00 00 00 00 00 00".to_string(), " ");
    let (header, body) = parse_dxb_header(&dxb);

	assert_eq!(header.version, 2);
	assert_eq!(header.size, 0);

	assert_eq!(header.signed, false);
	assert_eq!(header.encrypted, false);
	assert_eq!(header.routing.ttl, 0xff);
	assert_eq!(header.routing.priority, 1);

	assert_eq!(header.routing.sender, None);
	assert_eq!(header.scope_id, 3);
	assert_eq!(header.block_index, 4);
	assert_eq!(header.block_increment, 5);
	assert_eq!(header.block_type, DXBBlockType::REQUEST);
	assert_eq!(header.timestamp, 9);

	println!("{:#?}", header);

}

/**
 * test if a DXBHeader struct is correctly converted into a DXB buffer, and correctly converted back to a DXBHeader
 */
#[test]
pub fn generate_header() {

	let runtime = Runtime::new();

	// // dxb -> header -> dxb
    // let header_dxb = hex_to_buffer_advanced("01 64 02 00 00 ff 01 03 ff ff".to_string(), " ");
    // let (header, body) = parse_dxb_header(&header_dxb);
	// let gen_header_dxb = append_dxb_header(&header, body);
	// assert_eq!(buffer_to_hex(header_dxb), buffer_to_hex(gen_header_dxb));

	// header -> dxb -> header
	let header = DXBHeader {
		version: 2,
		size: 65535,
		signed: true,
		encrypted: true,

		block_type: DXBBlockType::REQUEST,
		scope_id: 22,
		block_index: 1,
		block_increment: 2,
		timestamp: 1234,

		flags: HeaderFlags {
			end_of_scope: true,
			allow_execute: true,
			device_type: 0
		},

		routing: RoutingInfo {
			ttl: 14,
			priority: 40,
			sender: Some(Endpoint::new_person("@theo", Endpoint::ANY_INSTANCE))
		}
	};
	let dxb = &hex_to_buffer_advanced("01 02 03".to_string(), " ");
	assert_eq!(parse_dxb_header(&append_dxb_header(runtime, &header, dxb)).0, header);

}