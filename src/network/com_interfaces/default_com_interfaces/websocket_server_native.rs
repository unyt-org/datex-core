use core::fmt;
use std::{error::Error, net::SocketAddr, sync::Mutex}; // FIXME no-std

use crate::{
    network::com_interfaces::websocket::{
        websocket_common::WebSocketError,
        websocket_server::{WebSocketServerError, WebSocketServerInterface},
    },
    stdlib::{cell::RefCell, collections::VecDeque, rc::Rc, sync::Arc},
};

use futures_util::{SinkExt, StreamExt};
use log::{debug, error, info};
use tokio::net::{TcpListener, TcpStream};
use tungstenite::Message;
use url::Url;

use crate::network::com_interfaces::websocket::{
    websocket_common::parse_url, websocket_server::WebSocket,
};
use tokio_tungstenite::accept_async;

pub struct WebSocketServerNative {
    // client: Option<Client<Box<dyn NetworkStream + Send>>>,
    address: Url,
    receive_queue: Arc<Mutex<VecDeque<u8>>>,
}

impl WebSocketServerNative {
    fn new(address: &str) -> Result<WebSocketServerNative, WebSocketError> {
        let address =
            parse_url(address).map_err(|_| WebSocketError::InvalidURL)?;
        Ok(WebSocketServerNative {
            receive_queue: Arc::new(Mutex::new(VecDeque::new())),
            address,
        })
    }

    async fn connect_async(
        address: &Url,
        receive_queue: Arc<Mutex<VecDeque<u8>>>,
    ) -> Result<(), WebSocketServerError> {
        let addr = format!(
            "{}:{}",
            address.host_str().unwrap(),
            address.port().unwrap()
        )
        .parse::<SocketAddr>()
        .map_err(|_| WebSocketServerError::InvalidPort)?;

        let listener = TcpListener::bind(&addr)
            .await
            .map_err(|_| WebSocketServerError::WebSocketError)?;
        info!("WebSocket server listening on ws://{}", addr);

        loop {
            let (stream, _) = listener
                .accept()
                .await
                .map_err(|_| WebSocketServerError::WebSocketError)?;
            // let queue = Arc::clone(&receive_queue);
            tokio::spawn(Self::handle_connection(
                stream,
                receive_queue.clone(),
            ));
        }
    }

    async fn handle_connection(
        stream: TcpStream,
        queue: Arc<Mutex<VecDeque<u8>>>,
    ) -> Result<(), WebSocketServerError> {
        let ws_stream = accept_async(stream)
            .await
            .map_err(|_| WebSocketServerError::WebSocketError)?;
        debug!("New connection established");

        let (mut write, mut read) = ws_stream.split();

        while let Some(msg) = read.next().await {
            let msg = msg.map_err(|_| WebSocketServerError::WebSocketError)?;
            match msg {
                Message::Binary(bin) => {
                    // pong TBD
                    queue.lock().unwrap().extend(bin.clone());
                    write.send(Message::Binary(bin.clone())).await.unwrap();
                }
                Message::Close(_) => {
                    println!("Client disconnected");
                    break;
                }
                _ => {}
            }
        }
        Ok(())
    }
}

impl WebSocket for WebSocketServerNative {
    fn connect(
        &mut self,
    ) -> Result<Arc<Mutex<VecDeque<u8>>>, WebSocketServerError> {
        let address = self.get_address();
        let receive_queue = self.receive_queue.clone();
        tokio::spawn(async move {
            if let Err(e) = WebSocketServerNative::connect_async(
                &address,
                receive_queue.clone(),
            )
            .await
            {
                error!("Server error: {}", e);
            }
        });
        Ok(self.receive_queue.clone())
    }

    fn get_address(&self) -> Url {
        self.address.clone()
    }

    fn send_data(&self, message: &[u8]) -> bool {
        todo!()
    }
}

impl WebSocketServerInterface<WebSocketServerNative> {
    pub fn new(
        port: u16,
    ) -> Result<WebSocketServerInterface<WebSocketServerNative>, WebSocketError>
    {
        let address = format!("127.0.0.1:{}", port);
        let websocket = WebSocketServerNative::new(&address.to_string())?;

        Ok(WebSocketServerInterface::new_with_web_socket_server(
            Rc::new(RefCell::new(websocket)),
        ))
    }
}
