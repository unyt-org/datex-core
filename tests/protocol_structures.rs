use binrw::{BinRead, BinWrite};
use datex_core::global::{
    dxb_block::DXBBlock,
    protocol_structures::{
        block_header::{BlockHeader, BlockType},
        encrypted_header::{self, EncryptedHeader},
        routing_header::{EncryptionType, RoutingHeader},
        serializable::Serializable,
    },
};
use datex_core::values::core_values::endpoint::{
    Endpoint, EndpointInstance, EndpointType,
};
use serde::Serialize;
use serde_json::ser::{Formatter, PrettyFormatter};
use std::{
    io::{Cursor, Seek, SeekFrom},
    str::FromStr,
};

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
    let routing_header = RoutingHeader::default()
        .with_sender(Endpoint {
            type_: EndpointType::Person,
            identifier: [0; 18],
            instance: EndpointInstance::Any,
        })
        .to_owned();

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
        routing_header: RoutingHeader::default(),
        ..DXBBlock::default()
    };

    let bytes = block.to_bytes().unwrap();

    let new_block = DXBBlock::from_bytes(bytes.as_slice()).unwrap();
    let new_bytes = new_block.to_bytes().unwrap();

    assert_eq!(bytes, new_bytes);
}

fn create_dxb_block_artifacts(block: &mut DXBBlock, name: &str) {
    use std::fs;
    use std::path::Path;
    let dir_path = Path::new("tests").join("structs").join(name);
    fs::create_dir_all(&dir_path).unwrap();
    let bin_path = dir_path.join("block.bin");
    let json_path = dir_path.join("block.json");

    let adjusted_block = block.recalculate_struct();
    fs::write(&bin_path, adjusted_block.to_bytes().unwrap()).unwrap();

    let mut buf = Vec::new();
    let mut ser = serde_json::Serializer::with_formatter(
        &mut buf,
        PrettyFormatter::with_indent(b"    "),
    );
    block.serialize(&mut ser).unwrap();

    let output = String::from_utf8(buf).unwrap();
    fs::write(&json_path, format!("{}\n", output)).unwrap();
}

#[test]
#[ignore = "Only run to create artifacts"]
pub fn dxb_blocks() {
    {
        const NAME: &str = "simple";
        let mut block = DXBBlock::default();
        block
            .routing_header
            .flags
            .set_encryption_type(EncryptionType::Encrypted);
        create_dxb_block_artifacts(&mut block, NAME);
    }

    {
        const NAME: &str = "receivers";
        let mut block = DXBBlock::default();
        block.set_receivers(vec![
            Endpoint::from_str("@jonas").unwrap(),
            Endpoint::from_str("@ben").unwrap(),
        ]);
        block.block_header.block_number = 42;
        block
            .block_header
            .flags_and_timestamp
            .set_block_type(BlockType::TraceBack);
        block
            .block_header
            .flags_and_timestamp
            .set_has_only_data(true);
        create_dxb_block_artifacts(&mut block, NAME);
    }

    {
        const NAME: &str = "receivers_with_keys";
        let mut block = DXBBlock::default();
        block.set_receivers(vec![
            (Endpoint::from_str("@jonas").unwrap(), [1u8; 512]),
            (Endpoint::from_str("@ben").unwrap(), [2u8; 512]),
        ]);
        block.block_header.block_number = 43;
        block
            .block_header
            .flags_and_timestamp
            .set_block_type(BlockType::TraceBack);
        block
            .block_header
            .flags_and_timestamp
            .set_has_only_data(true);
        create_dxb_block_artifacts(&mut block, NAME);
    }
}
