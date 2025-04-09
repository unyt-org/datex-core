use datex_core::network::com_interfaces::com_interface::ComInterface;
use datex_core::network::com_interfaces::websocket::websocket_server::WebSocketServerInterface;
use datex_core::stdlib::{cell::RefCell, rc::Rc};

use datex_core::network::com_interfaces::websocket::websocket_client::WebSocketClientInterface;

use crate::context::init_global_context;

#[test]
pub fn test_construct() {
    init_global_context();
    let client = WebSocketClientInterface::new("ws://localhost:8080").unwrap();
}

#[test]
pub fn test_client_connect() {
    init_global_context();

    let server = WebSocketServerInterface::new(1234).unwrap();

    let client =
        &mut WebSocketClientInterface::new("ws://localhost:8080").unwrap();
    client.connect().unwrap();
}
