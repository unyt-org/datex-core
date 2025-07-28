use binrw::{BinRead, BinWrite};
use datex_core::values::core_values::endpoint::{
    Endpoint, EndpointInstance, EndpointType,
};
use datex_core::global::{
    dxb_block::DXBBlock,
    protocol_structures::{
        block_header::BlockHeader,
        encrypted_header::{self, EncryptedHeader},
        routing_header::{Flags, ReceiverFlags, Receivers, RoutingHeader},
        serializable::Serializable,
    },
};
use std::io::{Cursor, Seek, SeekFrom};
// FIXME #214 no-std

#[test]
pub fn parse_encrypted_header() {
    let endpoint = Endpoint {
        type_: EndpointType::Person,
        identifier: [1; 18],
        instance: EndpointInstance::Any,
    };
    let encrypted_header = EncryptedHeader {
        on_behalf_of: Some(endpoint.clone()),
        flags: encrypted_header::Flags::new().with_has_on_behalf_of(true),
        ..EncryptedHeader::default()
    };
    let mut writer = Cursor::new(Vec::new());
    encrypted_header.write(&mut writer).unwrap();

    let mut reader = writer;
    reader.seek(SeekFrom::Start(0)).unwrap();

    let header_result = EncryptedHeader::read(&mut reader).unwrap();
    assert!(header_result.flags.has_on_behalf_of());
    assert!(header_result.clone().on_behalf_of.unwrap() == endpoint);
    assert!(
        header_result.to_bytes().unwrap()
            == encrypted_header.to_bytes().unwrap()
    );
}

#[test]
pub fn parse_block_header() {
    let block_header = BlockHeader::default();
    let mut writer = Cursor::new(Vec::new());
    block_header.write(&mut writer).unwrap();

    let mut reader = writer;
    reader.seek(SeekFrom::Start(0)).unwrap();

    let header_result = BlockHeader::read(&mut reader).unwrap();
    assert!(
        header_result.to_bytes().unwrap() == block_header.to_bytes().unwrap()
    );
}

#[test]
pub fn parse_routing_header() {
    let routing_header = RoutingHeader {
        version: 2,
        flags: Flags::new(),
        block_size_u16: Some(0),
        block_size_u32: None,
        sender: Endpoint {
            type_: EndpointType::Person,
            identifier: [0; 18],
            instance: EndpointInstance::Any,
        },
        receivers: Receivers {
            flags: ReceiverFlags::new()
                .with_has_endpoints(false)
                .with_has_pointer_id(false)
                .with_has_endpoint_keys(false),
            pointer_id: None,
            endpoints: None,
            endpoints_with_keys: None,
        },
        ..Default::default()
    };
    let mut writer = Cursor::new(Vec::new());
    routing_header.write(&mut writer).unwrap();

    let mut reader = writer;
    reader.seek(SeekFrom::Start(0)).unwrap();

    let header_result = RoutingHeader::read(&mut reader).unwrap();
    assert!(
        header_result.to_bytes().unwrap() == routing_header.to_bytes().unwrap()
    );
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

    let new_block = DXBBlock::from_bytes(bytes.as_slice()).unwrap();
    let new_bytes = new_block.to_bytes().unwrap();

    assert_eq!(bytes, new_bytes);
}
