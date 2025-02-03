use datex_core::datex_values::Endpoint;
use datex_core::global::dxb_block::DXBBlock;
use datex_core::global::dxb_header::{DXBBlockType, DXBHeader, HeaderFlags, RoutingInfo};
use datex_core::utils::buffers::hex_to_buffer_advanced;

// const CTX:&LoggerContext = &LoggerContext {log_redirect:None};

/**
 * test if dxb header is correctly parsed into a DXBHeader struct
 */
#[test]
pub fn parse_header() {
    // dxb -> header
    let dxb = hex_to_buffer_advanced(
        "01 64 02 00 00 ff 01 00 ff ff ff 03 00 00 00 04 00 05 00 00 01 09 00 00 00 00 00 00 00"
            .to_string(),
        " ",
    );
    let header_result = DXBHeader::from_bytes(&dxb);
    assert!(header_result.is_ok());
    let header = header_result.unwrap();

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
