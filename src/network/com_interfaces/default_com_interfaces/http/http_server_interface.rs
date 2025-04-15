use axum::body::Body;
use axum::extract::FromRef;
use axum::Extension;
use bytes::Bytes;

use axum::response::{IntoResponse, Response};
use futures::Stream;
use std::collections::{HashMap, VecDeque};
use std::convert::Infallible;
use std::future::Future;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::StreamExt;

use axum::{
    body::BodyDataStream,
    extract::{Path, State},
    response::sse::{Event, KeepAlive, Sse},
    routing::{get, post},
    Json, Router,
};
use log::{error, info, warn};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::spawn;
use tokio::sync::{broadcast, mpsc, RwLock};
use url::Url;

use crate::datex_values::Endpoint;
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
use crate::network::com_interfaces::socket_provider::MultipleSocketProvider;

use super::http_common::HTTPError;

pub struct HTTPServerNativeInterface {
    pub address: Url,
    info: ComInterfaceInfo,
    channels: Arc<RwLock<HashMap<String, broadcast::Sender<Bytes>>>>,
}

async fn stream_handler(
    Path(route): Path<String>,
    State(state): State<HTTPServerState>,
) -> Response {
    let map = state.channels.read().await;
    if let Some(sender) = map.get(&route) {
        let receiver = sender.subscribe();
        let stream = BroadcastStream::new(receiver);
        Response::builder()
            .header("Content-Type", "application/octet-stream")
            .header("Cache-Control", "no-cache")
            .body(axum::body::Body::from_stream(stream))
            .unwrap()
    } else {
        Response::builder()
            .status(404)
            .body("Channel not found".into())
            .unwrap()
    }
}

#[derive(Clone)]
struct HTTPServerState {
    channels: Arc<RwLock<HashMap<String, broadcast::Sender<Bytes>>>>,
}

impl MultipleSocketProvider for HTTPServerNativeInterface {
    fn provide_sockets(&self) -> Arc<Mutex<ComInterfaceSockets>> {
        self.get_sockets().clone()
    }
}

impl HTTPServerNativeInterface {
    pub async fn open(
        port: &u16,
    ) -> Result<HTTPServerNativeInterface, HTTPError> {
        let info = ComInterfaceInfo::new();
        let address: String = format!("http://127.0.0.1:{}", port);
        let address =
            Url::parse(&address).map_err(|_| HTTPError::InvalidAddress)?;

        let mut interface = HTTPServerNativeInterface {
            channels: Arc::new(RwLock::new(HashMap::new())),
            address,
            info,
        };
        interface.start().await?;
        Ok(interface)
    }

    pub async fn add_channel(&mut self, route: &str, endpoint: Endpoint) {
        let mut map = self.channels.write().await;
        if !map.contains_key(route) {
            let (tx, _) = broadcast::channel(100);
            map.insert(route.to_string(), tx);
            let socket = ComInterfaceSocket::new(
                self.get_uuid().clone(),
                InterfaceDirection::IN_OUT,
                1,
            );
            let socket_uuid = socket.uuid.clone();
            self.add_socket(Arc::new(Mutex::new(socket)));
            self.register_socket_endpoint(socket_uuid, endpoint, 0)
                .unwrap();
        }
    }
    pub async fn remove_channel(&mut self, route: &str) {
        let mut map = self.channels.write().await;
        if let Some(sender) = map.get(route) {
            map.remove(route);
        }
    }

    async fn start(&mut self) -> Result<(), HTTPError> {
        let address = self.address.clone();
        info!("Spinning up server at {}", address);

        let state = HTTPServerState {
            channels: self.channels.clone(),
        };
        let app = Router::new()
            .route("/:route/rx", get(stream_handler))
            .with_state(state.clone());

        let addr: SocketAddr = self
            .address
            .socket_addrs(|| None)
            .map_err(|_| HTTPError::InvalidAddress)?
            .first()
            .cloned()
            .ok_or(HTTPError::InvalidAddress)?;

        println!("HTTP server starting on http://{}", addr);
        tokio::spawn(async move {
            let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
            axum::serve(listener, app.into_make_service())
                .await
                .unwrap();
        });

        Ok(())
    }
}

impl ComInterface for HTTPServerNativeInterface {
    fn init_properties(&self) -> InterfaceProperties {
        InterfaceProperties {
            channel: "http".to_string(),
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
        let route = "test";
        let channels = self.channels.clone();
        Box::pin(async move {
            let map = channels.read().await;
            if let Some(sender) = map.get(route) {
                let _ = sender.send(Bytes::copy_from_slice(block));
                true
            } else {
                false
            }
        })
    }

    delegate_com_interface_info!();
}
