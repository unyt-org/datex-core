extern crate websocket;

use std::{collections::VecDeque, net::TcpStream, sync::{Arc, Mutex}};

use websocket::{
    sync::{stream::TlsStream, Client},
    ClientBuilder, Message,
};

use crate::network::com_interfaces::{
    com_interface_properties::{InterfaceDirection, InterfaceProperties},
    com_interface_socket::ComInterfaceSocket,
};

use super::super::com_interface::ComInterface;

type WSSClient = Client<TlsStream<TcpStream>>;

pub struct WebSocketClientInterface {
    client: WSSClient,
}

impl WebSocketClientInterface {
    pub fn new(address: &str) -> WebSocketClientInterface {
        let mut client = ClientBuilder::new(address)
            .unwrap()
            .connect_secure(None)
            .unwrap();

        for message in client.incoming_messages() {
            println!("Recv: {:?}", message.unwrap());
        }

        return WebSocketClientInterface { client };
    }
}

impl ComInterface for WebSocketClientInterface {
    fn send_block(&mut self, block: &[u8], socket: &ComInterfaceSocket) -> () {
        let message = Message::binary(block);
        self.client.send_message(&message).unwrap();
    }

    fn get_properties(&self) -> InterfaceProperties {
        InterfaceProperties {
            channel: "websocket".to_string(),
            name: None,
            direction: InterfaceDirection::IN_OUT,
            reconnect_interval: None,
            latency: 0,
            bandwidth: 1000,
            continuous_connection: true,
            allow_redirects: true,
        }
    }

   fn get_sockets(&self) -> std::rc::Rc<std::cell::RefCell<Vec<std::rc::Rc<std::cell::RefCell<ComInterfaceSocket>>>>> {
        todo!()
   }
}
