use datex_core::stdlib::{cell::RefCell, rc::Rc};

use datex_core::network::com_interfaces::websocket::websocket_client::WebSocketClientInterface;

use crate::context::init_global_context;

#[test]
pub fn test_construct() {
    init_global_context();
    let client = WebSocketClientInterface::new("ws://localhost:8080").unwrap();
}
