#[test]
pub fn tcp_client_interface() {
    // let interface = TCPClientInterface {};
}

#[test]
pub fn ws_client_interface() {
    // let mut interface = WebSocketClientInterface::new("wss://relay1.unyt.cc");

    // // send block
    // let header = DXBHeader {
    //     version: 2,
    //     size: 0,
    //     signed: false,
    //     encrypted: false,

    //     block_type: DXBBlockType::HELLO,
    //     scope_id: 1,
    //     block_index: 0,
    //     block_increment: 0,
    //     timestamp: 1234,

    //     flags: HeaderFlags {
    //         end_of_scope: true,
    //         allow_execute: true,
    //         device_type: 0,
    //     },

    //     routing: RoutingInfo {
    //         ttl: 14,
    //         priority: 40,
    //         sender: Some(Endpoint::new_person("@theo", Endpoint::ANY_INSTANCE)),
    //     },
    // };
    // let dxb = &hex_to_buffer_advanced("00".to_string(), " ");
    // let block = append_dxb_header(&header, dxb);
    // interface.send_block(&block)
}
