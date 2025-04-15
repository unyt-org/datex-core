use std::collections::{HashMap, VecDeque};
use std::future::Future;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::{
    extract::{Path, State},
    response::sse::{Event, KeepAlive, Sse},
    routing::{get, post},
    Json, Router,
};
use log::{error, info, warn};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::spawn;
use tokio::sync::{broadcast, mpsc};
use url::Url;

use crate::delegate_com_interface_info;
use crate::network::com_interfaces::com_interface::{
    ComInterface, ComInterfaceState,
};
use crate::network::com_interfaces::com_interface::{
    ComInterfaceInfo, ComInterfaceSockets, ComInterfaceUUID,
};
use crate::network::com_interfaces::com_interface_properties::{
    InterfaceDirection, InterfaceProperties,
};
use crate::network::com_interfaces::com_interface_socket::{
    ComInterfaceSocket, ComInterfaceSocketUUID,
};

use super::http_common::HTTPError;

pub struct HTTPServerNativeInterface {
    pub address: Url,
    clients: Arc<
        Mutex<
            HashMap<ComInterfaceSocketUUID, Arc<Mutex<mpsc::Sender<String>>>>,
        >,
    >,
    info: ComInterfaceInfo,
}

type SharedClients = Arc<
    Mutex<HashMap<ComInterfaceSocketUUID, Arc<Mutex<mpsc::Sender<String>>>>>,
>;

#[derive(Clone)]
struct HTTPServerState {
    clients: SharedClients,
    global_tx: broadcast::Sender<String>,
}

impl HTTPServerNativeInterface {
    pub async fn open(
        port: &u16,
    ) -> Result<HTTPServerNativeInterface, HTTPError> {
        let info = ComInterfaceInfo::new();
        let address: String = format!("http://127.0.0.1:{}", port);
        let address =
            Url::parse(&address).map_err(|_| HTTPError::InvalidURL)?;

        let mut interface = HTTPServerNativeInterface {
            address,
            info,
            clients: Arc::new(Mutex::new(HashMap::new())),
        };
        interface.start().await?;
        Ok(interface)
    }

    async fn start(&mut self) -> Result<(), HTTPError> {
        let address = self.address.clone();
        info!("Spinning up server at {}", address);

        let (global_tx, _) = broadcast::channel(100);
        let state = HTTPServerState {
            clients: self.clients.clone(),
            global_tx,
        };

        let app = Router::new()
            .route("/connect/:id", get(connect_handler))
            .route("/send/:id", post(send_to_client_handler))
            .route("/broadcast", post(broadcast_handler))
            .with_state(state);

        let addr: SocketAddr = self
            .address
            .socket_addrs(|| None)
            .map_err(|_| HTTPError::InvalidAddress)?
            .first()
            .cloned()
            .ok_or(HTTPError::InvalidAddress)?;

        println!("HTTP server starting on http://{}", addr);
        tokio::spawn(async move {
            axum::Server::bind(&addr)
                .serve(app.into_make_service())
                .await
                .unwrap();
        });

        Ok(())
    }

    async fn handle_client(
        mut rx: OwnedReadHalf,
        receive_queue: Arc<Mutex<VecDeque<u8>>>,
    ) {
        let mut buffer = [0u8; 1024];
        loop {
            match rx.read(&mut buffer).await {
                Ok(0) => {
                    warn!("Connection closed by peer");
                    break;
                }
                Ok(n) => {
                    info!("Received: {:?}", &buffer[..n]);
                    let mut queue = receive_queue.lock().unwrap();
                    queue.extend(&buffer[..n]);
                }
                Err(e) => {
                    error!("Failed to read from socket: {}", e);
                    break;
                }
            }
        }
    }
}

impl ComInterface for HTTPServerNativeInterface {
    fn init_properties(&self) -> InterfaceProperties {
        InterfaceProperties {
            channel: "tcp".to_string(),
            round_trip_time: Duration::from_millis(20),
            max_bandwidth: 1000,
            ..InterfaceProperties::default()
        }
    }
    fn close<'a>(&'a mut self) -> Pin<Box<dyn Future<Output = bool> + 'a>> {
        // TODO
        Box::pin(async move { true })
    }
    fn send_block<'a>(
        &'a mut self,
        block: &'a [u8],
        socket: ComInterfaceSocketUUID,
    ) -> Pin<Box<dyn Future<Output = bool> + 'a>> {
        let tx_queues = self.tx.clone();
        let tx_queues = tx_queues.lock().unwrap();
        let tx = tx_queues.get(&socket);
        if tx.is_none() {
            error!("Client is not connected");
            return Box::pin(async { false });
        }
        let tx = tx.unwrap().clone();
        Box::pin(async move { tx.lock().unwrap().write(block).await.is_ok() })
    }

    delegate_com_interface_info!();
}
