
use std::net::TcpStream;

use anyhow::Result;
use url::Url;
use websocket::{
    sync::{stream::TlsStream, Client}, ClientBuilder
};


use super::super::websocket_client::{parse_url, WebSocket, WebSocketClientInterface};

struct WebSocketNative {
	client: Client<TlsStream<TcpStream>>,
	address: Url
}

impl WebSocketNative {
	fn new(address: &str) -> Result<WebSocketNative> {

		let address = parse_url(address)?;

		let mut client = ClientBuilder::new(address.as_str())
			.unwrap()
			.connect_secure(None)
			.unwrap();

		for message in client.incoming_messages() {
			println!("Recv: {:?}", message.unwrap());
		}

		Ok(WebSocketNative { 
			client,
			address
		 })
	}
}

impl WebSocket for WebSocketNative {

	fn connect(&self) -> Result<()> {
		todo!()
	}

	fn send_data(&self, message: &[u8]) -> bool {
		todo!()
	}

	fn get_address(&self) -> Url {
		self.address.clone()
	}
}

impl WebSocketClientInterface<WebSocketNative> {
	pub fn new(address: &str) -> Result<WebSocketClientInterface<WebSocketNative>> {
		let websocket = WebSocketNative::new(address)?;
		
		Ok(WebSocketClientInterface::new_with_web_socket(websocket))
	}

}