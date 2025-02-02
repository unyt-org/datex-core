use std::io::{Cursor, Seek, SeekFrom};

use binrw::{BinRead, BinWrite};
use datex_core::global::{dxb_block::DXBBlock, protocol_structures::routing_header::{EndpointType, Flags, ReceiverFlags, Receivers, RoutingHeader, Sender, SignatureType}};

#[test]
pub fn parse_routing_header() {
	let routing_header = RoutingHeader {
		version: 2,
		ttl: 0,
		flags: Flags::new(),
		block_size_u16: Some(0),
		block_size_u32: None,
		scope_id: 0,
		block_index: 0,
		block_increment: 0,
		sender: Sender {
			sender_type: EndpointType::Person,
			sender_id: [0; 20],
		},
		receivers: Receivers {
			flags: ReceiverFlags::new()
				.with_has_endpoints(false)
				.with_has_pointer_id(false)
				.with_has_endpoint_keys(false),
			pointer_id: None,
			endpoints: None,
			endpoints_with_keys: None,
		}
	};
	let mut writer = Cursor::new(Vec::new());
	routing_header.write(&mut writer).unwrap();

	// Read our position back out of our Vec
	let mut reader = writer;
	reader.seek(SeekFrom::Start(0)).unwrap();

	println!("reader: {:?}\n", reader);
	
	let header_result = RoutingHeader::read(&mut reader);

	match header_result {
		Ok(header) => {
			println!("{:#?}", header);
		}
		Err(e) => {
			panic!("Error parsing header: {:?}", e);
		}
	}

	// }
}

#[test]
pub fn parse_dxb_block() {
	let block = DXBBlock {
		routing_header: RoutingHeader {
			version: 42,
			..RoutingHeader::default()
		},
		..DXBBlock::default()
	};

	let bytes = block.to_bytes().unwrap();

	println!("bytes: {:?}\n", bytes);

	let new_block = DXBBlock::from_bytes(bytes.as_slice()).unwrap();
	let new_bytes = new_block.to_bytes().unwrap();

	println!("parsed: {:#?}", new_block);

	assert_eq!(bytes, new_bytes);
}