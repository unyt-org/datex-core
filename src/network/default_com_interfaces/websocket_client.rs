extern crate websocket;

use std::{net::TcpStream};

use websocket::{ClientBuilder, Message, sync::{Client, stream::TlsStream}};

use crate::network::com_interface::ComInterface;


type WSSClient = Client<TlsStream<TcpStream>>;

pub struct WebSocketClientInterface {
	client: WSSClient
}

impl WebSocketClientInterface {
	pub fn new(address: &str) -> WebSocketClientInterface {
		let client = ClientBuilder::new(address)
			.unwrap()
			.connect_secure(None)
			.unwrap();

		// for message in client.incoming_messages() {
		// 	println!("Recv: {:?}", message.unwrap());
		// }

		return WebSocketClientInterface {
			client: client
		}
	}
}

impl ComInterface for WebSocketClientInterface {
    const NAME: &'static str = "ws_client";
    const IN: bool = true;
    const OUT: bool = true;
	const GLOBAL: bool = true;
	const VIRTUAL: bool = false;

	fn send_block(&mut self, block: &[u8]) -> () {
		let message = Message::binary(block);
		self.client.send_message(&message).unwrap();
    }
	
}